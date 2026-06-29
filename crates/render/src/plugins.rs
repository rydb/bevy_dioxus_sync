use std::time::Instant;

use bevy_app::prelude::*;
use bevy_render::{Render, RenderApp, RenderSystems, renderer::RenderDevice};
use vello::RendererOptions;

use crate::{panels::DioxusUiPanelsPlugin, *};

pub struct DioxusRenderPlugin;

impl Plugin for DioxusRenderPlugin {
    fn build(&self, app: &mut App) {
        let epoch = AnimationTime(Instant::now());

        let documents = HashMap::new();
        app.insert_non_send(DioxusDocuments(documents));
        
        app.add_plugins(DioxusUiPanelsPlugin);

        // Waker that sets a flag when dioxus futures become ready.
        // The flag is checked in update_uis to re-poll the document.
        struct DioxusWaker {
            flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
        }
        impl std::task::Wake for DioxusWaker {
            fn wake(self: std::sync::Arc<Self>) {
                self.flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        }
        let waker_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let waker = std::task::Waker::from(std::sync::Arc::new(DioxusWaker {
            flag: waker_flag.clone(),
        }));

        app.insert_non_send(waker);
        app.insert_non_send(DioxusWakerFlag(waker_flag));
        app.insert_resource(epoch);

        app
        .add_systems(Startup, setup_window_surface)
        .add_systems(Update, (recv_dioxus_messages, recompute_dioxus_ui_quad_surface, recompute_blitz_render_surfaces, update_uis).chain());
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
