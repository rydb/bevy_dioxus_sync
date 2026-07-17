use std::collections::HashMap;
use std::time::Instant;

use bevy_asset::{RenderAssetUsages, prelude::*};
use bevy_camera::visibility::RenderLayers;
use bevy_camera::{Camera, Camera3d, ClearColorConfig};
use bevy_derive::Deref;
use bevy_dioxus_interop::DioxusMessage;
use bevy_dioxus_tracing::{Level, debug, span, trace, warn};
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
use blitz_dom::local_name;
use blitz_traits::shell::ColorScheme;
use crossbeam_channel::{Receiver, Sender};
use dioxus_core::Element;
use dioxus_core_macro::{component, rsx};
use dioxus_hooks::{use_context, use_future, use_signal};
use dioxus_native::DioxusDocument;
use dioxus_signals::{ReadableExt, WritableExt};
use vello::{RenderParams, Renderer as VelloRenderer, peniko::color::AlphaColor};
use wgpu::{Extent3d, TextureDimension, TextureFormat};

use crate::panels::{DioxusPanels, DioxusPanelsReceiver};
use crate::worker::{VdomCommand, VdomResult, VdomThreadRegistry};

pub const SCALE_FACTOR: f32 = 1.0;
pub const COLOR_SCHEME: ColorScheme = ColorScheme::Light;

/// Multiplier applied to mesh dimensions to determine UI render resolution.
pub const RESOLUTION_SCALE: f32 = 500.0;

/// CSS class name used to mark DOM elements that consume input events.
pub const CATCH_EVENTS_CLASS: &str = "catch-events";

/// Walks up the DOM from a node checking for the catch-events class.
pub fn does_catch_events(dioxus_doc: &DioxusDocument, node_id: usize) -> bool {
    if let Some(node) = dioxus_doc.inner.borrow().get_node(node_id) {
        let class = node.attr(local_name!("class")).unwrap_or("");
        if class
            .split_whitespace()
            .any(|word| word == CATCH_EVENTS_CLASS)
        {
            true
        } else if let Some(parent) = node.parent {
            does_catch_events(dioxus_doc, parent)
        } else {
            false
        }
    } else {
        false
    }
}

/// Tracks whether the window overlay DOM consumed input in the previous frame.
#[derive(Resource, Default)]
pub struct WindowOverlayCatchState {
    pub caught_last_frame: bool,
}

pub(crate) mod net_provider;
pub mod panels;
pub mod plugins;
pub(crate) mod schedule;
pub mod worker;

/// Extraction-side mirror of texture handles, keyed by the quad entity.
#[derive(Resource, Default)]
struct ExtractedTextureImages(pub HashMap<Entity, Handle<Image>>);

/// root ui that all dioxus panels render inside of
#[component]
pub fn dioxus_ui() -> Element {
    let panel_receiver = use_context::<DioxusPanelsReceiver>();
    let mut panels = use_signal(|| DioxusPanels::default());

    // receive updates for panels
    use_future(move || {
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
    });
    rsx! {
        for panel in panels.read().cloned().panels {
            {panel()}
        }
    }
}

/// Marks camera as dioxus window ui camera
///
/// TODO: Add multi-window support
#[derive(Component)]
pub struct DioxusWindowUiCamera;

/// Marker for the main window entity that hosts the primary dioxus UI.
#[derive(Component)]
pub struct MainDioxusWindow;

/// Marks an entity as a DOM-backed render surface.
#[derive(Component)]
#[require(Mesh3d)]
pub struct DioxusUiQuad {
    pub handle: Option<Handle<Image>>,
    /// computed width and height of render surface based on attached bevy mesh
    pub computed_wh: Option<Vec2>,
    /// Half-extents of the quad in local mesh space.
    /// Used to convert world-space raycast hits into UV coordinates for picking.
    pub local_half_extents: Option<Vec2>,
}

impl Default for DioxusUiQuad {
    fn default() -> Self {
        Self {
            handle: None,
            computed_wh: None,
            local_half_extents: None,
        }
    }
}

/// Recompute dioxus ui quad surface whenever the associated mesh for it edited
fn recompute_dioxus_ui_quad_surface(
    mut surfaces: Query<(
        Entity,
        &Mesh3d,
        &mut DioxusUiQuad,
        Option<&DioxusUiResolution>,
    )>,
    meshes: Res<Assets<Mesh>>,
) {
    for (_e, surface, mut ui, resolution) in &mut surfaces {
        let id = surface.id();
        let Some(surface) = meshes.get(id) else {
            warn!("surface id not valid for? {}", id);
            continue;
        };

        let Some(VertexAttributeValues::Float32x3(positions)) =
            surface.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            warn!("surface doesn't have Float32x3 positions? {}", id);
            continue;
        };

        let Some(points) = positions.get(0..4) else {
            warn!(
                "shape does not have 4 points. Point total for {}: {}",
                id,
                positions.len()
            );
            continue;
        };

        if points.len() > 4 {
            warn!(
                "quads are computed from rectangles, not other shapes. Exiting early for performance. Expected 4 points for: {}, got: {}",
                id,
                positions.len()
            );
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
            if x > x_max {
                x_max = x;
            }
            if x < x_min {
                x_min = x;
            }
            if y > y_max {
                y_max = y;
            }
            if y < y_min {
                y_min = y;
            }
        }

        let half_extents = Some(Vec2 {
            x: (x_max - x_min) / 2.0,
            y: (y_max - y_min) / 2.0,
        });

        let (width, height) = if let Some(res) = resolution {
            (res.0 as f32, res.1 as f32)
        } else {
            (
                (x_max - x_min) * RESOLUTION_SCALE,
                (y_max - y_min) * RESOLUTION_SCALE,
            )
        };
        let new_wh = Some(Vec2 {
            x: width,
            y: height,
        });

        // Only change the quad if the underlying value actually changed
        if ui.computed_wh == new_wh && ui.local_half_extents == half_extents {
            continue;
        }

        ui.computed_wh = new_wh;
        ui.local_half_extents = half_extents;

        debug!("re-computed wh for {}: {:?}", _e, ui.computed_wh);
    }
}

/// Sends resize commands to VDOM workers when the ui quad dimensions change.
fn recompute_blitz_render_surfaces(
    quads: Query<(Entity, &DioxusUiQuad), Changed<DioxusUiQuad>>,
    registry: NonSend<VdomThreadRegistry>,
) {
    for (e, quad) in quads {
        let Some(wh) = quad.computed_wh else {
            continue;
        };
        let Some(worker) = registry.workers.get(&e) else {
            continue;
        };
        let _ = worker
            .cmd_tx
            .try_send(VdomCommand::Resize(wh.x as u32, wh.y as u32));
        trace!(
            "sent resize command for {}: {}x{}",
            e, wh.x as u32, wh.y as u32
        );
    }
}

#[derive(Debug)]
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

#[derive(Resource, Deref, Debug)]
struct MainWorldReceiver(Receiver<(Entity, RenderTexture)>);

#[derive(Resource, Deref, Debug)]
struct RenderWorldSender(Sender<(Entity, RenderTexture)>);

fn texture_getter_system(
    sender: Res<RenderWorldSender>,
    mut extracted_images: ResMut<ExtractedTextureImages>,
    gpu_images: Res<RenderAssets<GpuImage>>,
) {
    let mut processed: Vec<Entity> = Vec::new();

    for (entity, image_handle) in &extracted_images.0 {
        if let Some(gpu_image) = gpu_images.get(image_handle) {
            let _ = sender.send((
                *entity,
                RenderTexture {
                    texture_view: (*gpu_image.texture_view).clone(),
                    width: gpu_image.texture_descriptor.size.width,
                    height: gpu_image.texture_descriptor.size.height,
                },
            ));
            processed.push(*entity);
        }
    }

    for entity in processed {
        extracted_images.0.remove(&entity);
    }
}

#[derive(Resource)]
struct AnimationTime(Instant);

/// Sends a Poll command to every VDOM worker at the start of each frame.
fn dispatch_vdom_polls(
    mut registry: NonSendMut<VdomThreadRegistry>,
    animation_epoch: Res<AnimationTime>,
) {
    let animation_time = animation_epoch.0.elapsed().as_secs_f64();
    for (_entity, worker) in &mut registry.workers {
        let _ = worker.cmd_tx.try_send(VdomCommand::Poll { animation_time });
    }
}

/// Collects painted scenes from all VDOM workers and renders them to GPU
/// textures. Combines scene collection and rendering into one system to
/// avoid an intermediate resource for passing scenes.
fn collect_and_render_vdom_scenes(
    mut registry: NonSendMut<VdomThreadRegistry>,
    mut vello_renderer: NonSendMut<VelloRenderer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    receiver: Res<MainWorldReceiver>,
    images: Res<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut quad_query: Query<(Entity, &mut MeshMaterial3d<StandardMaterial>, &DioxusUiQuad)>,
    mut cached_textures: Local<HashMap<Entity, RenderTexture>>,
    mut catch_state: ResMut<WindowOverlayCatchState>,
) {
    // let _ = span!(Level::DEBUG, "total vdom(s) render time").entered();

    // Handle incoming GPU textures from the render world.
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
                    materials.get_mut(&mut mat.0).unwrap().base_color_texture =
                        Some(handle.clone());
                }
            }
        }
        cached_textures.insert(entity, texture);
    }
    let mut catch_state_this_frame = None;
    // Collect painted scenes from all workers and render them.
    for (entity, worker) in &mut registry.workers {
        // let span = span!(Level::DEBUG, "paint_scene collection", entity = %entity).entered();
        while let Ok(result) = worker.result_rx.try_recv() {
            match result {
                VdomResult::SceneReady {
                    scene,
                    width,
                    height,
                } => {
                    let Some(texture) = cached_textures.get(&entity) else {
                        continue;
                    };
                    vello_renderer
                        .render_to_texture(
                            render_device.wgpu_device(),
                            &render_queue.0,
                            &scene,
                            &texture.texture_view,
                            &RenderParams {
                                base_color: AlphaColor::TRANSPARENT,
                                width,
                                height,
                                antialiasing_method: vello::AaConfig::Area,
                            },
                        )
                        .expect("failed to render to texture");
                }
                VdomResult::ShutdownAck => {
                    debug!("vdom worker for {} acknowledged shutdown", entity);
                }
                VdomResult::InputCaught => {}
                VdomResult::HitTestResult { entity: _, caught } => {
                    if let Some(state) = catch_state_this_frame {
                        if state == true {
                            continue;
                        }
                    }
                    catch_state_this_frame = Some(caught)
                }
            }
        }
        // span.exit();
    }
    if let Some(result) = catch_state_this_frame {
        catch_state.caught_last_frame = result;
    }
}

/// Cleans up worker threads when their associated entities are despawned.
fn cleanup_vdom_workers(
    mut removed: RemovedComponents<DioxusUiQuad>,
    mut registry: NonSendMut<VdomThreadRegistry>,
) {
    for entity in removed.read() {
        if let Some(mut worker) = registry.workers.remove(&entity) {
            let _ = worker.cmd_tx.send(VdomCommand::Shutdown);
            if let Some(handle) = worker.thread.take() {
                let _ = handle.join();
            }
            debug!("cleaned up vdom worker for {}", entity);
        }
    }
}

const WINDOW_UI_RENDER_LAYER: RenderLayers = RenderLayers::layer(1);

/// Marks an entity with ui quad as a window for window specific systems.
#[derive(Component)]
pub struct DioxusWindowUiQuad;

/// Sets the logical CSS viewport resolution for a dioxus UI quad.
///
/// TODO: decide best practice on how to correlate this with dioxus ui quad surface
#[derive(Component, Clone, Copy)]
pub struct DioxusUiResolution(pub u32, pub u32);

/// initialize textures for quads
fn initialize_textures_for_quads(
    quads: Query<(Entity, &mut DioxusUiQuad), Without<MeshMaterial3d<StandardMaterial>>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    for (e, mut quad) in quads {
        // initialize texture after computed_wh is created
        let Some(wh) = quad.computed_wh else { continue };

        let image = create_ui_texture(wh.x as u32, wh.y as u32);

        let handle = images.add(image);
        commands
            .entity(e)
            .insert(MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(handle.clone()),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            })));
        quad.handle = Some(handle);
        debug!("Initialized texture for: {}", e);
    }
}

/// Set up window surface + camera for window.
fn setup_window_surface(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    // mut images: ResMut<Assets<Image>>,
    windows: Query<(Entity, &Window)>,
) {
    let Some((_e, window)) = windows.iter().next() else {
        warn!("no window found, skipping dioxus ui initialization");
        return;
    };
    if windows.iter().len() > 1 {
        warn!(
            "window setup only implemented for one window. TODO: decide how to resolve multiple windows, for now. Skipping"
        );
        return;
    }

    let wh = window.physical_size();
    let aspect = wh.x as f32 / wh.y as f32;

    // Derive frustum height so that viewport resolution matches the window:
    let fov = std::f32::consts::PI / 4.0;
    let visible_height = wh.y as f32 / RESOLUTION_SCALE;
    let visible_width = visible_height * aspect;
    let distance = visible_height / (2.0 * (fov / 2.0).tan());

    let _ui_entity = commands
        .spawn((
            Mesh3d(meshes.add(Rectangle::new(visible_width, visible_height))),
            Transform::from_xyz(0.0, 0.0, 0.0),
            DioxusUiQuad {
                handle: None,
                computed_wh: None,
                local_half_extents: None,
            },
            DioxusUiResolution(wh.x, wh.y),
            DioxusPanels::default(),
            DioxusWindowUiQuad,
            WINDOW_UI_RENDER_LAYER,
        ))
        .id();

    let _camera = commands.spawn((
        Camera3d::default(),
        Camera {
            order: isize::MAX,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, distance),
        WINDOW_UI_RENDER_LAYER,
        DioxusWindowUiCamera,
    ));
}

/// Updates the window UI quad and camera when the window is resized.
fn handle_window_resize(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut window_quad: Query<
        (Entity, &mut DioxusUiResolution, &mut DioxusUiQuad),
        With<DioxusWindowUiQuad>,
    >,
    mut camera: Query<&mut Transform, With<DioxusWindowUiCamera>>,
    window: Single<&Window>,
    mut last_size: Local<UVec2>,
) {
    let wh = window.physical_size();
    if *last_size == UVec2::ZERO {
        *last_size = wh;
        return;
    }
    if wh != *last_size {
        let fov = std::f32::consts::PI / 4.0;
        let aspect = wh.x as f32 / wh.y as f32;
        let visible_height = wh.y as f32 / RESOLUTION_SCALE;
        let visible_width = visible_height * aspect;
        let distance = visible_height / (2.0 * (fov / 2.0).tan());

        *last_size = wh;

        for (entity, mut resolution, mut quad) in &mut window_quad {
            *resolution = DioxusUiResolution(wh.x, wh.y);
            let new_image = create_ui_texture(wh.x, wh.y);
            quad.handle = Some(images.add(new_image));
            commands.entity(entity).insert(Mesh3d(
                meshes.add(Rectangle::new(visible_width, visible_height)),
            ));
        }

        for mut transform in &mut camera {
            transform.translation.z = distance;
        }
    }
}
