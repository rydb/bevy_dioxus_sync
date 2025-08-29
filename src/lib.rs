use bevy_ecs::{
    component::{ComponentHooks, Immutable, StorageType},
    prelude::*,
    schedule::ScheduleLabel,
    system::ScheduleSystem,
};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    sync::Arc,
};

use crate::{
    systems::{DioxusPanelUpdates, PanelUpdate, PanelUpdateKind},
    traits::{DioxusElementMarker, ErasedSubGenericMap},
};

pub mod dioxus_in_bevy_plugin;
pub(crate) mod systems;
pub mod traits;
pub mod ui;

pub mod event_sync;
pub mod hooks;
pub mod render;
pub mod resource_sync;

pub struct SenderReceiver<T: Send + Sync + 'static> {
    pub sender: Sender<T>,
    pub receiver: Receiver<T>,
}

/// An untyped hashmap that resolved typed entries by their type id.
pub type ArcAnytypeMap = HashMap<TypeId, Arc<dyn Any + Send + Sync>>;

pub type BoxAnyTypeMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;

#[derive(Clone, Default, Debug, TransparentWrapper)]
#[repr(transparent)]
pub struct TxChannelRegistry(ArcAnytypeMap);

impl ErasedSubGenericMap for TxChannelRegistry {
    type Generic<T: Send + Sync + 'static> = Sender<T>;
}

#[derive(Clone, Default, Debug, TransparentWrapper)]
#[repr(transparent)]
pub struct RxChannelRegistry(ArcAnytypeMap);

impl ErasedSubGenericMap for RxChannelRegistry {
    type Generic<T: Send + Sync + 'static> = Receiver<T>;
}

/// Bevy side channel for giving [`T`] to dioxus
#[derive(Resource)]
struct BevyTxChannel<T>(pub Sender<T>);

/// Dioxus side channel for receiving [`T`] from bevy.
#[derive(Resource)]
struct BevyRxChannel<T>(pub Receiver<T>);

/// Dioxus side channel for sending [`T`] to bevy
pub struct DioxusTxChannel<T>(pub Sender<T>);

/// Bevy side channel for receiving [`T`] from dioxus.
#[derive(Resource)]
pub struct DioxusRxChannel<T>(pub Receiver<T>);

/// Component that marks an entity as a dioxus panel
#[derive(Clone, Debug)]
pub struct DioxusPanel {
    pub(crate) element_marker: Arc<dyn DioxusElementMarker>,
}

impl DioxusPanel {
    pub fn new<T: DioxusElementMarker>(element: T) -> Self {
        Self {
            element_marker: Arc::new(element),
        }
    }
}

impl Component for DioxusPanel {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    /// to change the panel on this entity, insert a new one.
    type Mutability = Immutable;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, hook| {
            let Some(value) = world.entity(hook.entity).get::<Self>() else {
                warn!(
                    "could not get {:#} on {:#}",
                    type_name::<Self>(),
                    hook.entity
                );
                return;
            };
            let value = value.clone();
            let mut panel_updates = world.get_resource_mut::<DioxusPanelUpdates>().unwrap();
            // warn!(
            //     "pushing panel update for {:#} to {:#?}",
            //     hook.entity,
            //     PanelUpdateKind::Add(value.clone())
            // );
            panel_updates.0.push(PanelUpdate {
                key: hook.entity,
                value: PanelUpdateKind::Add(value),
            })
        });
        hooks.on_remove(|mut world, hook| {
            let mut panel_updates = world.get_resource_mut::<DioxusPanelUpdates>().unwrap();
            panel_updates.0.push(PanelUpdate {
                key: hook.entity,
                value: PanelUpdateKind::Remove,
            })
        });
    }
}
pub struct ResourceUpdates {}

fn add_systems_through_world<T>(
    world: &mut World,
    schedule: impl ScheduleLabel,
    systems: impl IntoScheduleConfigs<ScheduleSystem, T>,
) {
    let mut schedules = world.get_resource_mut::<Schedules>().unwrap();
    if let Some(schedule) = schedules.get_mut(schedule) {
        schedule.add_systems(systems);
    }
}

pub type BoxSync = Box<dyn Any + Send + Sync + 'static>;
