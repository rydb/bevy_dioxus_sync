use std::{any::{type_name, TypeId}, sync::{mpsc::Receiver, Arc}};

use bevy_ecs::{component::{ComponentHooks, Immutable, Mutable, StorageType}, prelude::*, world::CommandQueue};
use bevy_log::warn;
use bevy_platform::collections::{HashMap, HashSet};
use crossbeam_channel::Sender;
use dioxus::core::Element;
use std::fmt::Debug;

use crate::{dioxus_in_bevy_plugin::DioxusCommandQueueRx, traits::DioxusElementMarker, DioxusPanel};

#[derive(Debug)]
pub struct PanelUpdate {
    pub(crate) key: Entity,
    pub(crate) value: PanelUpdateKind
}

#[derive(Debug)]
pub enum PanelUpdateKind {
    Add(DioxusPanel),
    Remove,
}

#[derive(Resource, Debug, Default)]
pub struct DioxusPanelUpdates(pub(crate) Vec<PanelUpdate>);


pub fn read_dioxus_command_queues(
    world: &mut World
) {
    let receiver = world.get_resource_mut::<DioxusCommandQueueRx>().unwrap().0.clone();
    while let Ok(mut command_queue) = receiver.try_recv() {
        world.commands().append(&mut command_queue);        
    }
}