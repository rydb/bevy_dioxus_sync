use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, world::CommandQueue};

use blitz_dom::DocumentConfig;
use crossbeam_channel::{Receiver, Sender};
use dioxus::prelude::*;
use dioxus_native_dom::DioxusDocument;

use crate::{
    ErasedSubGenericMap,
    event_sync::plugins::DioxusEventSyncPlugin,
    render::plugins::DioxusRenderPlugin,
    systems::{DioxusPanelUpdates, read_dioxus_command_queues},
    ui::dioxus_app,
};

pub struct DioxusPlugin {}
/// props for [`DioxusPlugin`]'s dioxus app.
#[derive(Clone)]
pub struct DioxusProps {
    pub(crate) dioxus_panel_updates: Receiver<DioxusPanelUpdates>,
    pub(crate) command_queues_tx: Sender<CommandQueue>,
}

#[derive(Resource)]
pub struct DioxusCommandQueueRx(pub Receiver<CommandQueue>);

#[derive(Resource)]
pub struct DioxusPanelUpdatesSender(Sender<DioxusPanelUpdates>);

impl Plugin for DioxusPlugin {
    fn build(&self, app: &mut App) {
        let (dioxus_panel_updates_tx, dioxus_panel_updates_rx) =
            crossbeam_channel::unbounded::<DioxusPanelUpdates>();
        let (command_queues_tx, command_queues_rx) = crossbeam_channel::unbounded::<CommandQueue>();

        let props = DioxusProps {
            dioxus_panel_updates: dioxus_panel_updates_rx,
            command_queues_tx: command_queues_tx,
        };
        app.add_plugins(DioxusRenderPlugin);
        app.add_plugins(DioxusEventSyncPlugin);
        app.init_resource::<DioxusPanelUpdates>();

        app.insert_resource(DioxusCommandQueueRx(command_queues_rx));
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
        app.add_systems(PreUpdate, read_dioxus_command_queues);
    }
}

pub fn push_panel_updates(
    mut panel_updates: ResMut<DioxusPanelUpdates>,
    panel_update_sender: ResMut<DioxusPanelUpdatesSender>,
) {
    let mut updates = Vec::new();

    updates.extend(panel_updates.0.drain(..));

    panel_update_sender.0.send(DioxusPanelUpdates(updates));
}
