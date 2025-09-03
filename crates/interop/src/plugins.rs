use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_app::prelude::*;


use super::*;

/// plugin for setting up minimal channels for interop
pub struct DioxusBevyInteropPlugin {
    pub command_tx: Sender<CommandQueue>,
    pub command_rx: Receiver<CommandQueue>,
}
impl DioxusBevyInteropPlugin {
    pub fn new() -> Self{
        let (command_queues_tx, command_queues_rx) = crossbeam_channel::unbounded::<CommandQueue>();
        Self {
            command_rx: command_queues_rx,
            command_tx: command_queues_tx,
        }
    }
}




impl Plugin for DioxusBevyInteropPlugin {
    fn build(&self, app: &mut App) {
        app
        .insert_resource(DioxusCommandQueueRx(self.command_rx.clone()));
        app.add_systems(PreUpdate, read_dioxus_command_queues);

    }
}