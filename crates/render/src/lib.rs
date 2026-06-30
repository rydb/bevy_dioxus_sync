use std::collections::{HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyrender_vello::VelloScenePainter;
use bevy_asset::{RenderAssetUsages, prelude::*};
use bevy_camera::visibility::RenderLayers;
use bevy_camera::{Camera, Camera3d, ClearColorConfig};
use bevy_camera::{OrthographicProjection, Projection, ScalingMode};
use bevy_color::Color;
use bevy_derive::Deref;
use bevy_dioxus_interop::{DioxusDocuments, DioxusMessage};
use bevy_dioxus_tracing::{debug, warn};
use bevy_ecs::prelude::*;
use bevy_image::prelude::*;
use bevy_material::AlphaMode;
use bevy_math::prelude::*;
use bevy_mesh::{Mesh, Mesh3d, VertexAttributeValues};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_render::{
    Extract,
    render_asset::RenderAssets,
    renderer::{RenderDevice, RenderQueue},
    texture::GpuImage,
};
use bevy_transform::components::Transform;
use bevy_utils::default;
use bevy_window::prelude::*;
use blitz_dom::Document;
use blitz_paint::paint_scene;
use blitz_traits::shell::{ColorScheme, Viewport};
use crossbeam_channel::{Receiver, Sender};
use dioxus_core::Element;
use dioxus_core_macro::{component, rsx};
use dioxus_devtools::DevserverMsg;
use dioxus_hooks::{use_context, use_future, use_signal};
use dioxus_signals::{ReadableExt, WritableExt};
use vello::{RenderParams, Renderer as VelloRenderer, Scene, peniko::color::AlphaColor};
use wgpu::{Extent3d, TextureDimension, TextureFormat};

use crate::panels::{DioxusPanels, DioxusPanelsReceiver};

pub const SCALE_FACTOR: f32 = 1.0;
pub const COLOR_SCHEME: ColorScheme = ColorScheme::Light;

/// Placeholder const for dioxus animations.
/// TODO: implement this.
pub const ANIMATION_TIME_PLACEHOLDER: f32 = 0.0;
pub mod plugins;
pub mod panels;
pub(crate) mod net_provider;

/// Extraction-side mirror of texture handles, keyed by the quad entity.
#[derive(Resource, Default)]
struct ExtractedTextureImages(pub HashMap<Entity, Handle<Image>>);


/// root ui that all dioxus panels render inside of
#[component]
pub fn dioxus_ui() -> Element {
    let panel_receiver = use_context::<DioxusPanelsReceiver>();
    let mut panels = use_signal(|| DioxusPanels::default());
    

    // recieve updates for panels
    use_future(move || {
        {
        let value = panel_receiver.clone();
        async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            std::thread::spawn(move || {
                while let Ok(updated_panels) = value.0.recv() {
                    if tx.send(updated_panels).is_err() {
                        break; 
                    }
                }
            });

            loop {
                tokio::select! {
                    Some(updated_panels) = rx.recv() => {
                        *panels.write() = updated_panels;
                    }
                }
            }
        }
        }
    });
    rsx! {
        for panel in panels.read().cloned().panels {
            {panel()}
        }
    }
}

/// Marker for the camera that the dioxus UI should follow.
#[derive(Component)]
pub struct DioxusUiCamera;

/// Marker for the main window entity that hosts the primary dioxus UI.
#[derive(Component)]
pub struct MainDioxusWindow;

/// Marks an entity as a DOM-backed render surface.
#[derive(Component)]
#[require(Mesh3d)]
pub struct DioxusUiQuad {
    pub handle: Option<Handle<Image>>,
    /// computed width and height of render surface based on attached bevy mesh
    pub(crate) computed_wh: Option<Vec2>
}

impl Default for DioxusUiQuad {
    fn default() -> Self {
        Self { handle: None, computed_wh: None }
    }
}

/// Recompute dioxus ui quad surface whenever the associated mesh for it edited
fn recompute_dioxus_ui_quad_surface(
    mut surfaces: Query<(Entity, &Mesh3d, &mut DioxusUiQuad)>,
    meshes: Res<Assets<Mesh>>,
) {
    for (_e, surface, mut ui) in &mut surfaces {
        let id = surface.id();
        let Some(surface) = meshes.get(id) else {
            warn!("surface id not valid for? {}", id);
            continue;
        };

        let Some(VertexAttributeValues::Float32x3(positions)) = surface.attribute(Mesh::ATTRIBUTE_POSITION) else {
            warn!("surface doesn't have Float32x3 positions? {}", id);
            continue;
        };

        let Some(points) = positions.get(0..4) else {
            warn!("shape does not have 4 points. Point total for {}: {}", id, positions.len());
            continue;
        };

        if points.len() > 4 {
            warn!("quads are computed from rectangles, not other shapes. Exiting early for performance. Expected 4 points for: {}, got: {}", id, positions.len());
            continue;
        }

        let first = points[0];

        let mut x_max = first[0];
        let mut x_min = first[0];
        let mut y_max = first[1];
        let mut y_min = first[1];

        for point in points.iter().skip(1) {
            let x = point[0];
            let y = point[1];
            if x > x_max { x_max = x; }
            if x < x_min { x_min = x; }
            if y > y_max { y_max = y; }
            if y < y_min { y_min = y; }
        }

        let width = x_max - x_min;
        let height = y_max - y_min;
        let new_wh = Some(Vec2 { x: width, y: height });

        // Only change the quad if the underlying value actually changed
        if ui.computed_wh == new_wh {
            continue;
        }

        ui.computed_wh = new_wh;

        debug!("re-computed wh for {}: {:?}", _e, ui.computed_wh);
    }
}

/// recomputes the render surface for blitz after the ui quad has been re-computed 
fn recompute_blitz_render_surfaces(
    quads: Query<(Entity, &DioxusUiQuad), Changed<DioxusUiQuad>>,
    mut dioxus_docs: NonSendMut<DioxusDocuments>
) {
    for (e, quad) in quads {
        let Some(wh) = quad.computed_wh else {
            continue;
        };
        let Some(doc) = dioxus_docs.0.get_mut(&e) else {
            warn!("no doc found for: {}", e);
            continue;
        };

        let mut doc = doc.document.inner.as_ref().borrow_mut();
        let mut view_port = doc.viewport_mut();
        view_port.window_size = (wh.x as u32, wh.y as u32);

        debug!("re-computed view-port for {}: {:?}", e, view_port.window_size);

    }
}

struct RenderTexture {
    pub texture_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

pub fn create_ui_texture(width: u32, height: u32) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8; 4],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = wgpu::TextureUsages::COPY_DST
        | wgpu::TextureUsages::STORAGE_BINDING
        | wgpu::TextureUsages::TEXTURE_BINDING;
    image
}

fn extract_texture_images(
    mut commands: Commands,
    quad_query: Extract<Query<(Entity, &DioxusUiQuad)>>,
    mut last_handles: Local<HashMap<Entity, Handle<Image>>>,
    extracted_images: Option<Res<ExtractedTextureImages>>,
) {
    let mut to_extract = HashMap::new();

    for (entity, quad) in &quad_query {
        let Some(handle) = &quad.handle else {
            continue;
        };
        let prev_still_pending = extracted_images
            .as_ref()
            .map_or(false, |e| e.0.contains_key(&entity));
        let handle_changed = last_handles.get(&entity) != Some(handle);

        if prev_still_pending && !handle_changed {
            continue;
        }

        to_extract.insert(entity, handle.clone());
        last_handles.insert(entity, handle.clone());
    }

    if !to_extract.is_empty() {
        commands.insert_resource(ExtractedTextureImages(to_extract));
    }
}

#[derive(Resource, Deref)]
struct MainWorldReceiver(Receiver<(Entity, RenderTexture)>);

#[derive(Resource, Deref)]
struct RenderWorldSender(Sender<(Entity, RenderTexture)>);

fn texture_getter_system(
    sender: Res<RenderWorldSender>,
    mut extracted_images: ResMut<ExtractedTextureImages>,
    gpu_images: Res<RenderAssets<GpuImage>>,
) {
    let mut processed: Vec<Entity> = Vec::new();

    for (entity, image_handle) in &extracted_images.0 {
        if let Some(gpu_image) = gpu_images.get(image_handle) {
            let _ = sender.send((*entity, RenderTexture {
                texture_view: (*gpu_image.texture_view).clone(),
                width: gpu_image.texture_descriptor.size.width,
                height: gpu_image.texture_descriptor.size.height,
            }));
            processed.push(*entity);
        }
    }

    for entity in processed {
        extracted_images.0.remove(&entity);
    }
}

#[derive(Resource)]
struct AnimationTime(Instant);

/// Flag set by the dioxus waker when a future becomes ready.
/// Cleared after the document is polled so the next wake can be detected.
#[derive(Clone, Deref)]
struct DioxusWakerFlag(Arc<AtomicBool>);

/// recieve dioxus messages
fn recv_dioxus_messages(
    mut dioxus_docs: NonSendMut<DioxusDocuments>,
    waker: NonSendMut<std::task::Waker>,
) {
    for (_,info) in &mut dioxus_docs.0 {
        while let Ok(msg) = info.messages_recv.try_recv() {
            match msg {
                DioxusMessage::Devserver(devserver_msg) => match devserver_msg {
                    dioxus_devtools::DevserverMsg::HotReload(hotreload_message) => {
                        dioxus_devtools::apply_changes(&info.document.vdom, &hotreload_message);
                        for asset_path in &hotreload_message.assets {
                            if let Some(url) = asset_path.to_str() {
                                info.document.inner.borrow_mut().reload_resource_by_href(url);
                            }
                        }
                    }
                    dioxus_devtools::DevserverMsg::FullReloadStart => {}
                    _ => {}
                },
                DioxusMessage::CreateHeadElement(el) => {
                    info.document.create_head_element(&el.name, &el.attributes, &el.contents);
                    info.document.poll(Some(std::task::Context::from_waker(&waker)));
                }
                DioxusMessage::ResourceLoad(resource) => {
                    info.document.inner.borrow_mut().load_resource(blitz_dom::net::ResourceLoadResponse {
                        request_id: 0,
                        node_id: None,
                        resolved_url: None,
                        result: Ok(resource.clone()),
                    });
                }
            };
        }
    }
}

fn update_uis(
    mut dioxus_docs: NonSendMut<DioxusDocuments>,
    waker: NonSendMut<std::task::Waker>,
    waker_flag: NonSend<DioxusWakerFlag>,
    mut vello_renderer: NonSendMut<VelloRenderer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    receiver: Res<MainWorldReceiver>,
    animation_epoch: Res<AnimationTime>,
    images: Res<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut quad_query: Query<(
        Entity,
        &mut MeshMaterial3d<StandardMaterial>,
        &DioxusUiQuad,
    )>,
    mut cached_textures: Local<HashMap<Entity, RenderTexture>>,
) {
    cached_textures.retain(|entity, _| quad_query.contains(*entity));

    while let Ok((entity, texture)) = receiver.try_recv() {
        for (quad_entity, mut mat, quad) in quad_query.iter_mut() {
            if quad_entity != entity {
                continue;
            }
            let Some(handle) = &quad.handle else {
                continue;
            };
            if let Some(img) = images.get(handle) {
                let sz = img.texture_descriptor.size;
                if texture.width == sz.width && texture.height == sz.height {
                    if let Some(info) = dioxus_docs.0.get_mut(&entity) {
                        info.document.inner.borrow_mut().set_viewport(
                            Viewport::new(texture.width, texture.height, SCALE_FACTOR, COLOR_SCHEME),
                        );
                    }
                    materials.get_mut(&mut mat.0).unwrap().base_color_texture = Some(handle.clone());
                }
            }
        }
        cached_textures.insert(entity, texture);
    }

    for (entity, info) in &mut dioxus_docs.0 {
        let Some(texture) = cached_textures.get(entity) else {
            continue;
        };

        // Poll until no more async work is flagged as ready.
        loop {
            waker_flag.0.store(false, Ordering::SeqCst);
            info.document.poll(Some(std::task::Context::from_waker(&waker)));
            if !waker_flag.0.load(Ordering::SeqCst) {
                break;
            }
        }

        let animation_time = animation_epoch.0.elapsed().as_secs_f64();
        info.document.inner.borrow_mut().resolve(animation_time);

        let mut scene = Scene::new();
        paint_scene(
            &mut VelloScenePainter::new(&mut scene),
            &mut *info.document.inner.borrow_mut(),
            SCALE_FACTOR as f64,
            texture.width,
            texture.height,
            0,
            0,
        );
        vello_renderer
            .render_to_texture(
                render_device.wgpu_device(),
                &render_queue.0,
                &scene,
                &texture.texture_view,
                &RenderParams {
                    base_color: AlphaColor::TRANSPARENT,
                    width: texture.width,
                    height: texture.height,
                    antialiasing_method: vello::AaConfig::Area,
                },
            )
            .expect("failed to render to texture");
    }
}

const WINDOW_UI_RENDER_LAYER: RenderLayers = RenderLayers::layer(1);

#[derive(Component)]
pub struct DioxusWindowUiQuad;

/// Set up window surface + camera for window.
fn setup_window_surface(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<(Entity, &Window)>,
) {
    let Some((_e, window)) = windows.iter().next() else {
        warn!("no window found, skipping dioxus ui initialization");
        return
    };
    if windows.iter().len() > 1 {
        warn!("window setup only implemented for one window. TODO: decide how to resolve multiple windows, for now. Skipping");
        return;
    }

    let wh = window.physical_size();
    let image = create_ui_texture(wh.x, wh.y);
    let handle = images.add(image);

    // Size the quad to match the window aspect ratio so the UI fills the view.
    let aspect = wh.x as f32 / wh.y as f32;
    let quad_h = 2.0;
    let quad_w = quad_h * aspect;

    let _ui_entity = commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(quad_w, quad_h))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(handle.clone()),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        DioxusUiQuad {
            handle: Some(handle),
            computed_wh: None,
        },
        DioxusPanels::default(),
        DioxusWindowUiQuad,
        WINDOW_UI_RENDER_LAYER,
    )).id();

    let _camera = commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical { viewport_height: quad_h },
            ..OrthographicProjection::default_2d()
        }),
        Camera {
            order: isize::MAX,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 4.0),
        WINDOW_UI_RENDER_LAYER,
        DioxusUiCamera,
    ));
}
