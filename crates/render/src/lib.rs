use std::time::Instant;

use anyrender_vello::VelloScenePainter;
use bevy_asset::{RenderAssetUsages, prelude::*};
use bevy_camera::{Camera, Camera2d};
use bevy_derive::Deref;
use bevy_ecs::prelude::*;
use bevy_image::prelude::*;
use bevy_log::{debug, warn};
use bevy_math::prelude::*;
use bevy_mesh::{Mesh, Mesh2d};
use bevy_render::{
    Extract,
    prelude::*,
    render_asset::RenderAssets,
    render_graph::{self, NodeRunError, RenderGraphContext, RenderLabel},
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::GpuImage,
};
use bevy_sprite::prelude::*;
use bevy_sprite_render::{ColorMaterial, MeshMaterial2d};
use bevy_transform::components::Transform;
use bevy_utils::default;
use bevy_window::prelude::*;
use blitz_dom::Document;
use blitz_paint::paint_scene;
use blitz_traits::shell::{ColorScheme, Viewport};
use crossbeam_channel::{Receiver, Sender};
use dioxus_native::{CustomPaintSource, DioxusDocument};
use rustc_hash::FxHashMap;
use vello::{RenderParams, Renderer as VelloRenderer, Scene, peniko::color::AlphaColor};
use wgpu::{Extent3d, TextureDimension, TextureFormat};

pub const SCALE_FACTOR: f32 = 1.0;
pub const COLOR_SCHEME: ColorScheme = ColorScheme::Light;
 
/// placeholder const for dioxus animations
/// TODO: implement this
pub const ANIMATION_TIME_PLACEHOLDER: f32 = 0.0;
pub mod plugins;

#[derive(Resource)]
pub struct TextureImage(pub Handle<Image>);

#[derive(Resource)]
pub struct ExtractedTextureImage(pub Option<Handle<Image>>);

#[derive(Component)]
pub struct DioxusUiQuad;

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

pub fn extract_texture_image(
    mut commands: Commands,
    texture_image: Extract<Option<Res<TextureImage>>>,
    mut last_texture_image: Local<Option<Handle<Image>>>,
) {
    if let Some(texture_image) = texture_image.as_ref() {
        if let Some(last_texture_image) = &*last_texture_image {
            if last_texture_image == &texture_image.0 {
                return;
            }
        }
        commands.insert_resource(ExtractedTextureImage(Some(texture_image.0.clone())));
        *last_texture_image = Some(texture_image.0.clone());
    }
}

#[derive(Resource, Deref)]
struct MainWorldReceiver(Receiver<RenderTexture>);

#[derive(Resource, Deref)]
struct RenderWorldSender(Sender<RenderTexture>);

#[derive(Debug, PartialEq, Eq, Clone, Hash, RenderLabel)]
struct TextureGetterNode;

#[derive(Default)]
struct TextureGetterNodeDriver;

impl render_graph::Node for TextureGetterNodeDriver {
    fn update(&mut self, world: &mut World) {
        // Get the GPU texture from the texture image, and send it to the main world
        if let Some(sender) = world.get_resource::<RenderWorldSender>() {
            if let Some(image) = world
                .get_resource::<ExtractedTextureImage>()
                .and_then(|e| e.0.as_ref())
            {
                if let Some(gpu_images) = world
                    .get_resource::<RenderAssets<GpuImage>>()
                    .and_then(|a| a.get(image))
                {
                    let _ = sender.send(RenderTexture {
                        texture_view: (*gpu_images.texture_view).clone(),
                        width: gpu_images.size.width,
                        height: gpu_images.size.height,
                    });
                    if let Some(mut extracted_image) =
                        world.get_resource_mut::<ExtractedTextureImage>()
                    {
                        // Reset the image, so it is not sent again, unless it changes
                        extracted_image.0 = None;
                    }
                }
            }
        }
    }
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        _world: &World,
    ) -> Result<(), NodeRunError> {
        Ok(())
    }
}

#[derive(Resource)]
struct AnimationTime(Instant);

#[allow(clippy::too_many_arguments)]
fn update_ui(
    mut dioxus_doc: NonSendMut<DioxusDocument>,
    waker: NonSendMut<std::task::Waker>,
    vello_renderer: Option<NonSendMut<VelloRenderer>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    receiver: Res<MainWorldReceiver>,
    animation_epoch: Res<AnimationTime>,
    mut cached_texture: Local<Option<RenderTexture>>,
) {
    while let Ok(texture) = receiver.try_recv() {
        *cached_texture = Some(texture);
    }

    if let (Some(texture), Some(mut vello_renderer)) = ((*cached_texture).as_ref(), vello_renderer)
    {
        let context = std::task::Context::from_waker(&waker);
        // Poll the vdom
        dioxus_doc.poll(Some(context));

        // Refresh the document
        let animation_time = animation_epoch.0.elapsed().as_secs_f64();
        dioxus_doc.resolve(animation_time);

        // Create a `vello::Scene` to paint into
        let mut scene = Scene::new();

        // Paint the document
        paint_scene(
            &mut VelloScenePainter::new(&mut scene),
            &dioxus_doc,
            SCALE_FACTOR as f64,
            texture.width,
            texture.height,
        );

        // Render the `vello::Scene` to the Texture using the `VelloRenderer`
        vello_renderer
            .render_to_texture(
                render_device.wgpu_device(),
                render_queue.into_inner(),
                &scene,
                &texture.texture_view,
                &RenderParams {
                    base_color: AlphaColor::TRANSPARENT,
                    width: texture.width,
                    height: texture.height,
                    antialiasing_method: vello::AaConfig::Msaa16,
                },
            )
            .expect("failed to render to texture");
    }
}



fn setup_ui(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut dioxus_doc: NonSendMut<DioxusDocument>,
    mut animation_epoch: ResMut<AnimationTime>,
    windows: Query<&Window>,
) {
    let window = windows
        .iter()
        .next()
        .expect("Should have at least one window");
    let width = window.physical_width();
    let height = window.physical_height();

    debug!("Initial window size: {}x{}", width, height);

    // Set the initial viewport
    animation_epoch.0 = Instant::now();
    dioxus_doc.set_viewport(Viewport::new(width, height, SCALE_FACTOR, COLOR_SCHEME));
    dioxus_doc.resolve(0.0);

    // Create Bevy Image from the texture data
    let image = create_ui_texture(width, height);
    let handle = images.add(image);

    // Create a quad to display the texture
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial2d(materials.add(ColorMaterial {
            texture: Some(handle.clone()),
            ..default()
        })),
        Transform::from_scale(Vec3::new(width as f32, height as f32, 0.0)),
        DioxusUiQuad,
    ));
    commands.spawn((
        Camera2d,
        Camera {
            order: isize::MAX,
            ..default()
        },
    ));

    commands.insert_resource(TextureImage(handle));
}
