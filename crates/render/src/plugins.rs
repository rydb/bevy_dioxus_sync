use std::time::Instant;

use bevy_app::prelude::*;
use bevy_render::{Render, RenderApp, RenderSystems, renderer::RenderDevice};
use vello::RendererOptions;

use crate::panels::{initialize_vdoms, sync_dioxus_ui_with_panels};
use crate::schedule::{
    DioxusRenderMain, DioxusRenderSchedule, DioxusRenderScheduleAccumulator,
    DioxusRenderScheduleTimestep,
};
use crate::worker::VdomThreadRegistry;
use crate::*;

pub struct DioxusRenderPlugin {
    pub fps_cap: u32,
}

impl Plugin for DioxusRenderPlugin {
    fn build(&self, app: &mut App) {
        let epoch = AnimationTime(Instant::now());

        app.add_schedule(Schedule::new(DioxusRenderSchedule));

        app.insert_non_send(VdomThreadRegistry::default());
        app.insert_resource(epoch);

        app.add_systems(Startup, setup_window_surface)
            .add_systems(PreUpdate, initialize_vdoms)
            .add_systems(
                DioxusRenderSchedule,
                (
                    cleanup_vdom_workers,
                    handle_window_resize,
                    sync_dioxus_ui_with_panels,
                    recompute_dioxus_ui_quad_surface,
                    recompute_blitz_render_surfaces,
                    initialize_textures_for_quads,
                    dispatch_vdom_polls,
                    collect_and_render_vdom_scenes,
                )
                    .chain(),
            );
        app.insert_resource(DioxusRenderScheduleAccumulator::default());
        app.insert_resource(DioxusRenderScheduleTimestep::from_fps(self.fps_cap));
        app.add_systems(Update, DioxusRenderMain::run_dioxus_render_main);
    }
    fn finish(&self, app: &mut App) {
        // Add the UI rendrer
        let render_app = app.sub_app(RenderApp);
        let render_device = render_app.world().resource::<RenderDevice>();
        let device = render_device.wgpu_device();
        let vello_renderer = VelloRenderer::new(device, RendererOptions::default()).unwrap();
        app.insert_non_send(vello_renderer);

        // Setup communication between main world and render world, to send
        // and receive the texture
        let (s, r) = crossbeam_channel::unbounded();
        app.insert_resource(MainWorldReceiver(r));
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(bevy_render::ExtractSchedule, extract_texture_images);
        render_app.insert_resource(RenderWorldSender(s));
        render_app.insert_resource(ExtractedTextureImages::default());

        // Add a system to get the GPU texture after assets are prepared
        render_app.add_systems(
            Render,
            texture_getter_system.after(RenderSystems::PrepareAssets),
        );
    }
}
