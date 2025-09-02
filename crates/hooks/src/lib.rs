use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use bevy_ecs::{prelude::*, schedule::ScheduleLabel, system::ScheduleSystem};

use crate::traits::{ArcAnytypeMap, ErasedSubGenericMap};

// pub mod asset_handle;
pub mod asset_single;
pub mod component_single;
pub mod traits;

// pub mod one_component_kind;


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


pub struct BevyDioxusIO<B, D = B> {
    pub bevy_tx: Sender<B>,
    pub bevy_rx: Receiver<D>,
    pub dioxus_tx: Sender<D>,
    pub dioxus_rx: Receiver<B>,
}

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
