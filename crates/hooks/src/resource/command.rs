use bevy_app::prelude::*;
use bevy_dioxus_interop::{add_systems_through_world, BevyDioxusIO, BevyRxChannel, BevyTxChannel};
use bevy_ecs::prelude::*;
use bevy_log::warn;
use crossbeam_channel::{Receiver, Sender};

use crate::DioxusTxRx;

pub enum InsertDefaultResource<T: Resource + Clone> {
    No,
    Yes(T),
}

/// Command to register dioxus bevy interop for a given resource.
#[derive(Clone)]
pub struct RequestBevyResource<T: Resource + Clone> 
    where
        T: Resource + Clone
{
    pub(crate) channels: BevyDioxusIO<(T, Option<()>)>
}

impl<T> Default for RequestBevyResource<T> 
    where
        T: Resource + Clone
{
    fn default() -> Self {
        Self { channels: Default::default() }
    }
}


impl<T> DioxusTxRx<T> for RequestBevyResource<T> 
    where
        T: Resource + Clone
{
    fn txrx(&self) -> BevyDioxusIO<(T, Option<()>), (T, Option<()>)> {
        self.channels.clone()
    }
} 

// impl<T:  Resource + Clone> Into<DioxusTxrX<T, ()>> for RequestBevyResource<T> {
//     fn into(self) -> DioxusTxrX<T, ()> {
//         DioxusTxrX { dioxus_tx: self.dioxus_tx, dioxus_rx: self.dioxus_rx }
//     }
// }

// impl<T: Resource + Clone> Default for RequestBevyResource<T> {
//     fn default() -> Self {
//         let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<T>();
//         let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<T>();
//         Self {
//             bevy_tx,
//             bevy_rx,
//             dioxus_tx,
//             dioxus_rx
//         }
//     }
// }

impl<T: Resource + Clone> Command for RequestBevyResource<T> {
    fn apply(self, world: &mut World) -> () {
        world.insert_resource(BevyTxChannel(self.channels.bevy_tx));
        world.insert_resource(BevyRxChannel(self.channels.bevy_rx));

        add_systems_through_world(
            world,
            Update,
            send_resource_update::<T>.run_if(resource_changed::<T>),
        );
        add_systems_through_world(world, Update, receive_resource_update::<T>);
    }
}

fn send_resource_update<T: Resource + Clone>(
    resource: Res<T>,
    bevy_tx: ResMut<BevyTxChannel<T>>,
    // bevy_rx: ResMut<BevyRxChannel<T>>,
) {
    let _ = bevy_tx
        .0
        .send(resource.clone())
        .inspect_err(|err| warn!("could not send resource: {:#}", err));
}

fn receive_resource_update<T: Resource + Clone>(
    mut resource: ResMut<T>,
    bevy_rx: ResMut<BevyRxChannel<T>>,
    // bevy_rx: ResMut<BevyRxChannel<T>>,
) {
    let Ok(new_res) = bevy_rx.0.try_recv() else {
        return;
    };
    *resource = new_res;
}
