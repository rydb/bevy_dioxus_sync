use bevy_app::prelude::*;
use bevy_dioxus_messages::plugins::DioxusEventSyncPlugin;
use bevy_dioxus_render::plugins::DioxusRenderPlugin;
use dioxus_bevy_signals::{CommandQueueSender, DioxusBevyMirrorPlugin};
use dioxus_core::{Element, ScopeId, VirtualDom, provide_context};

use crate::{InitialWindowPanel, setup_initial_window_ui};


pub struct DioxusPlugin {
    /// how many times per second does dioxus refresh info from bevy.
    pub bevy_info_refresh_fps: u32,
    pub main_window_ui: Option<fn() -> Element>,
}

impl Plugin for DioxusPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(InitialWindowPanel(self.main_window_ui));

        let dioxus_signals_mirror_plugin = DioxusBevyMirrorPlugin {
            dioxus_sync_fps: self.bevy_info_refresh_fps,
            bevy_command_txrx: Default::default(),
        };

        app.add_systems(PostStartup, setup_initial_window_ui);
        app.add_plugins(DioxusEventSyncPlugin);
        app.add_plugins(dioxus_signals_mirror_plugin);
        app.add_plugins(DioxusRenderPlugin);
    }
}
