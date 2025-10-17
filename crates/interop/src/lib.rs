use std::{any::{Any, TypeId}, collections::HashMap, sync::Arc};

use bevy_ecs::{prelude::*, schedule::ScheduleLabel, system::ScheduleSystem, world::CommandQueue};
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};

use crate::traits::ErasedSubGenericMap;

pub mod traits;
pub mod plugins;

/// Bevy side channel for giving [`T`] to dioxus
#[derive(Resource)]
pub struct BevyTxChannel<T>(pub Sender<T>);

/// Dioxus side channel for receiving [`T`] from bevy.
#[derive(Resource)]
pub struct BevyRxChannel<T>(pub Receiver<T>);

/// Dioxus side channel for sending [`T`] to bevy
pub struct DioxusTxChannel<T>(pub Sender<T>);

/// Bevy side channel for receiving [`T`] from dioxus.
#[derive(Resource)]
pub struct DioxusRxChannel<T>(pub Receiver<T>);

#[derive(Resource)]
pub struct DioxusCommandQueueRx(pub Receiver<CommandQueue>);


/// An untyped hashmap that resolved typed entries by their type id.
pub type ArcAnytypeMap = HashMap<TypeId, Arc<dyn Any + Send + Sync>>;

pub type BoxAnyTypeMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;

pub type BoxAnySignalTypeMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;

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

#[derive(Clone)]
pub struct BevyDioxusIO<A, B = A> {
    pub bevy_tx: Sender<A>,
    pub bevy_rx: Receiver<B>,
    pub dioxus_tx: Sender<B>,
    pub dioxus_rx: Receiver<A>,
}

impl<A, B> Default for BevyDioxusIO<A, B> {
    fn default() -> Self {
        let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<A>();
        let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<B>();

        Self {
            bevy_tx,
            bevy_rx,
            dioxus_tx,
            dioxus_rx,
        }
    }
}

pub fn add_systems_through_world<T>(
    world: &mut World,
    schedule: impl ScheduleLabel,
    systems: impl IntoScheduleConfigs<ScheduleSystem, T>,
) {
    let mut schedules = world.get_resource_mut::<Schedules>().unwrap();
    if let Some(schedule) = schedules.get_mut(schedule) {
        schedule.add_systems(systems);
    }
}

#[derive(Clone)]
pub struct BevyCommandQueueTx(pub Sender<CommandQueue>);

/// refresh rate for info sent to dioxus.
#[derive(Clone)]
pub struct InfoRefershRateMS(pub u64);

impl Default for InfoRefershRateMS {
    fn default() -> Self {
        Self(30)
    }
}

pub fn read_dioxus_command_queues(world: &mut World) {
    let receiver = world
        .get_resource_mut::<DioxusCommandQueueRx>()
        .unwrap()
        .0
        .clone();
    while let Ok(mut command_queue) = receiver.try_recv() {
        world.commands().append(&mut command_queue);
    }
}
