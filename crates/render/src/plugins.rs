use std::time::Instant;

use bevy_app::prelude::*;
use bevy_render::{render_graph::RenderGraph, renderer::RenderDevice, RenderApp};
use vello::RendererOptions;

use crate::*;

pub struct DioxusRenderPlugin;

impl Plugin for DioxusRenderPlugin {
    fn build(&self, app: &mut App) {
        let epoch = AnimationTime(Instant::now());
        
        
        // Dummy waker
        struct NullWake;
        impl std::task::Wake for NullWake {
            fn wake(self: std::sync::Arc<Self>) {}
        }
        let waker = std::task::Waker::from(std::sync::Arc::new(NullWake));

        app.insert_non_send_resource(waker);

        app.insert_resource(epoch);
        app
        .add_systems(Startup, setup_ui)
        .add_systems(Update, update_ui)
        ;
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
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(bevy_render::ExtractSchedule, extract_texture_image);
        render_app.insert_resource(RenderWorldSender(s));

        // Add a render graph node to get the GPU texture
        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(TextureGetterNode, TextureGetterNodeDriver);
        graph.add_node_edge(bevy_render::graph::CameraDriverLabel, TextureGetterNode);
    }
}
