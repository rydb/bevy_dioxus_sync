use bevy_ecs::prelude::*;
use bevy_log::warn;
use std::fmt::Debug;

use crate::{DioxusPanel, dioxus_in_bevy_plugin::DioxusCommandQueueRx};

#[derive(Debug)]
pub struct PanelUpdate {
    pub(crate) key: Entity,
    pub(crate) value: PanelUpdateKind,
}

#[derive(Debug)]
pub enum PanelUpdateKind {
    Add(DioxusPanel),
    Remove,
}

#[derive(Resource, Debug, Default)]
pub struct DioxusPanelUpdates(pub(crate) Vec<PanelUpdate>);

pub fn read_dioxus_command_queues(world: &mut World) {
    let receiver = world
        .get_resource_mut::<DioxusCommandQueueRx>()
        .unwrap()
        .0
        .clone();
    while let Ok(mut command_queue) = receiver.try_recv() {
        warn!("ammending command queue: {:#?}", command_queue);
        world.commands().append(&mut command_queue);
    }
}
