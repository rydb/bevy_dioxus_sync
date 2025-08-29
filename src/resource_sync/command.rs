use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_log::warn;
use crossbeam_channel::{Receiver, Sender};

use crate::{BevyRxChannel, BevyTxChannel, add_systems_through_world};

pub enum InsertDefaultResource<T: Resource + Clone> {
    No,
    Yes(T),
}

/// Command to register dioxus bevy interop for a given resource.
pub(crate) struct RequestBevyResource<T: Resource + Clone> {
    // default_resource: InsertDefaultResource<T>,

    pub(crate) dioxus_tx: Sender<T>,
    pub(crate) dioxus_rx: Receiver<T>,
    pub(crate) bevy_tx: Sender<T>,
    pub(crate) bevy_rx: Receiver<T>,
}

impl<T: Resource + Clone> RequestBevyResource<T> {
    pub fn new(
        //default_resource: InsertDefaultResource<T>
    ) -> Self {
        let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<T>();
        let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<T>();

        Self {
            //default_resource,
            dioxus_tx,
            dioxus_rx,
            bevy_tx,
            bevy_rx,
        }
    }
}

impl<T: Resource + Clone> Command for RequestBevyResource<T> {
    fn apply(self, world: &mut World) -> () {
        world.insert_resource(BevyTxChannel(self.bevy_tx));
        world.insert_resource(BevyRxChannel(self.bevy_rx));

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
