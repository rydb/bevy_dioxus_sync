use bevy_app::prelude::*;
use bevy_render::{prelude::*, renderer::RenderDevice, RenderApp};
use vello::{Renderer, RendererOptions};

use crate::render::*;

pub struct DioxusRenderPlugin;

impl Plugin for DioxusRenderPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(Startup, setup_ui)
        .add_systems(Update, update_ui);


    }
    fn finish(&self, app: &mut App) {
        // Add the UI rendrer
        let render_app = app.sub_app(RenderApp);
        let render_device = render_app.world().resource::<RenderDevice>();
        let device = render_device.wgpu_device();
        let vello_renderer = VelloRenderer::new(device, RendererOptions::default()).unwrap();
        app.insert_non_send_resource(vello_renderer);

        // Setup communication between main world and render world, to send
        // and receive the texture
        let (s, r) = crossbeam_channel::unbounded();
        app.insert_resource(MainWorldReceiver(r));
        let render_app = app.sub_app_mut(bevy_render::RenderApp);
        render_app.add_systems(bevy_render::ExtractSchedule, extract_texture_image);
        render_app.insert_resource(RenderWorldSender(s));

        // Add a render graph node to get the GPU texture
        let mut graph = render_app.world_mut().resource_mut::<bevy_render::render_graph::RenderGraph>();
        graph.add_node(TextureGetterNode, TextureGetterNodeDriver);
        graph.add_node_edge(bevy_render::graph::CameraDriverLabel, TextureGetterNode);
    }
}