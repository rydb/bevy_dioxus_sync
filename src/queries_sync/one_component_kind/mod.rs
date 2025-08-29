use std::{any::type_name, collections::{HashMap, HashSet}, sync::Arc, thread::scope};

use async_std::task::sleep;
use bevy_app::Update;
use bevy_ecs::{prelude::*, query::QueryData, world::CommandQueue};
use bevy_log::{info, warn};
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus::{hooks::{use_context, use_future}, signals::{Signal, SignalSubscriberDrop, SyncSignal, UnsyncStorage, WritableExt, WriteLock}};

use crate::{add_systems_through_world, dioxus_in_bevy_plugin::DioxusProps, queries_sync::BevyDioxusIO, traits::ErasedSubGenericComponentsMap, BevyRxChannel, BevyTxChannel, BoxAnyTypeMap};


pub mod command;
pub mod hook;

pub type EntityComponentQueue<T> = AddRemoveQueues<Entity, T>;
pub struct AddRemoveQueues<T, U> {
    pub add: HashMap<T, U>,
    pub remove: HashSet<T>
}

impl<T, U> Default for AddRemoveQueues<T, U> {
    fn default() -> Self {
        Self { add: HashMap::default(), remove: HashSet::default() }
    }
}
pub struct BevyQueryComponents<T: Component> {
    components: HashMap<Entity, T>,
    pub(crate) query_read: Receiver<EntityComponentQueue<T>>,
    query_write: Sender<EntityComponentQueue<T>>,
}
