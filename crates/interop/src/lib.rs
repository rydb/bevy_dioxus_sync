use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use bevy_ecs::{prelude::*, schedule::ScheduleLabel, system::ScheduleSystem, world::CommandQueue};
use crossbeam_channel::{Receiver as CrossBeamReceiver, Sender as CrossBeamSender};
use tokio::sync::broadcast::{self, Receiver as TokioReceiver, Sender as TokioSender};

pub mod plugins;

/// Bevy side channel for giving [`T`] to dioxus
#[derive(Resource)]
pub struct BevyTxChannel<T>(pub TokioSender<T>);

/// Dioxus side channel for receiving [`T`] from bevy.
#[derive(Resource)]
pub struct BevyRxChannel<T>(pub CrossBeamReceiver<T>);

#[derive(Resource)]
pub struct DioxusCommandQueueRx(pub CrossBeamReceiver<CommandQueue>);

/// An untyped hashmap that resolved typed entries by their type id.
pub type ArcAnytypeMap = HashMap<TypeId, Arc<dyn Any + Send + Sync>>;

pub type BoxAnyTypeMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;

pub type BoxAnySignalTypeMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;

#[derive(Clone, Debug)]
pub enum InfoPacket<T, U, V> {
    Update(InfoUpdate<T, U, V>),
    Request(StatusUpdate),
}

#[derive(Clone, Debug)]
pub enum StatusUpdate {
    /// request a refresh
    RequestRefresh,
}

/// data to/from dioxus
#[derive(Clone, Debug)]
pub struct InfoUpdate<T, U, V> {
    pub update: T,
    pub index: Option<U>,
    pub additional_info: Option<V>,
}

/// channels for bevy dioxus interop.
pub struct BevyDioxusIO<
    // bevy -> dioxus
    A: Clone,
    Index,
    AdditionalInfo: Clone,
    // dioxus -> bevy
    C: Clone = A,
> {
    pub bevy_tx: TokioSender<InfoPacket<A, Index, AdditionalInfo>>,
    pub bevy_rx: CrossBeamReceiver<InfoPacket<C, Index, AdditionalInfo>>,
    pub dioxus_tx: CrossBeamSender<InfoPacket<C, Index, AdditionalInfo>>,
    pub dioxus_rx: TokioReceiver<InfoPacket<A, Index, AdditionalInfo>>,
}

impl<A: Clone, Index: Clone, C: Clone, AdditionalInfo: Clone> Default
    for BevyDioxusIO<A, Index, AdditionalInfo, C>
{
    fn default() -> Self {
        let (bevy_tx, dioxus_rx) = broadcast::channel(16);
        let (dioxus_tx, bevy_rx) =
            crossbeam_channel::unbounded::<InfoPacket<C, Index, AdditionalInfo>>();

        Self {
            bevy_tx,
            bevy_rx,
            dioxus_tx,
            dioxus_rx,
        }
    }
}

/// adds the given system(s), [`T`], to the given system schedule.
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

/// command queue tx for dioxus -> bevy.
#[derive(Clone)]
pub struct BevyCommandQueueTx(pub CrossBeamSender<CommandQueue>);

/// refresh rate for info sent to dioxus. Lower means more frequent refreshes.
#[derive(Clone)]
pub struct InfoRefershRateMS(pub u64);

impl Default for InfoRefershRateMS {
    fn default() -> Self {
        Self(30)
    }
}

pub(crate) fn read_dioxus_command_queues(world: &mut World) {
    let receiver = world
        .get_resource_mut::<DioxusCommandQueueRx>()
        .unwrap()
        .0
        .clone();
    while let Ok(mut command_queue) = receiver.try_recv() {
        world.commands().append(&mut command_queue);
    }
}
