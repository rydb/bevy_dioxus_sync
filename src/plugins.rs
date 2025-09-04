use bevy_app::prelude::*;
use bevy_dioxus_events::plugins::DioxusEventSyncPlugin;
use bevy_dioxus_interop::plugins::DioxusBevyInteropPlugin;
use bevy_dioxus_render::plugins::DioxusRenderPlugin;
use bevy_ecs::{prelude::*, world::CommandQueue};

use bevy_log::warn;
use blitz_dom::DocumentConfig;
use crossbeam_channel::{Receiver, Sender};
use dioxus_core::VirtualDom;
use dioxus_native_dom::DioxusDocument;

use crate::{
    systems::DioxusPanelUpdates, ui::dioxus_app
};

pub struct DioxusPlugin {
    /// how many times per second does dioxus refresh info from bevy.
    pub bevy_info_refresh_fps: u16,
}
/// props for [`DioxusPlugin`]'s dioxus app.
#[derive(Clone)]
pub struct DioxusProps {
    pub(crate) dioxus_panel_updates: Receiver<DioxusPanelUpdates>,
    pub(crate) command_queues_tx: Sender<CommandQueue>,
    pub(crate) fps: u16,
}



#[derive(Resource)]
pub struct DioxusPanelUpdatesSender(Sender<DioxusPanelUpdates>);

impl Plugin for DioxusPlugin {
    fn build(&self, app: &mut App) {
        
        let bevy_dioxus_interop_plugin = DioxusBevyInteropPlugin::new();
        let (dioxus_panel_updates_tx, dioxus_panel_updates_rx) =
            crossbeam_channel::unbounded::<DioxusPanelUpdates>();

        let props = DioxusProps {
            dioxus_panel_updates: dioxus_panel_updates_rx,
            command_queues_tx: bevy_dioxus_interop_plugin.command_tx.clone(),
            fps: self.bevy_info_refresh_fps,
        };
        app.add_plugins(bevy_dioxus_interop_plugin);
        app.add_plugins(DioxusRenderPlugin);
        app.add_plugins(DioxusEventSyncPlugin);
        app.init_resource::<DioxusPanelUpdates>();

        app.insert_resource(DioxusPanelUpdatesSender(dioxus_panel_updates_tx));

        // Create the dioxus virtual dom and the dioxus-native document
        // The viewport will be set in setup_ui after we get the window size
        let vdom = VirtualDom::new_with_props(dioxus_app, props);
        // FIXME add a NetProvider
        let mut dioxus_doc = DioxusDocument::new(vdom, DocumentConfig::default());
        dioxus_doc.initial_build();
        dioxus_doc.resolve();

        // Dummy waker
        struct NullWake;
        impl std::task::Wake for NullWake {
            fn wake(self: std::sync::Arc<Self>) {}
        }
        let waker = std::task::Waker::from(std::sync::Arc::new(NullWake));

        app.insert_non_send_resource(dioxus_doc);
        app.insert_non_send_resource(waker);
        app.add_systems(
            PreUpdate,
            push_panel_updates.run_if(resource_changed::<DioxusPanelUpdates>),
        );
    }
}

pub fn push_panel_updates(
    mut panel_updates: ResMut<DioxusPanelUpdates>,
    panel_update_sender: ResMut<DioxusPanelUpdatesSender>,
) {
    let mut updates = Vec::new();

    updates.extend(panel_updates.0.drain(..));

    let _ = panel_update_sender
        .0
        .send(DioxusPanelUpdates(updates))
        .inspect_err(|err| warn!("{:#}", err));
}
