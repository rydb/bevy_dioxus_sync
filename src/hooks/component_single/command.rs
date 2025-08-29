use bevy_app::Update;
use bevy_ecs::{component::Mutable, prelude::*};
use bevy_log::warn;
use std::marker::PhantomData;

use crate::{BevyRxChannel, BevyTxChannel, add_systems_through_world};
use crossbeam_channel::{Receiver, Sender};

/// Command to register dioxus bevy interop for a given resource.
pub(crate) struct RequestBevyComponentSingleton<T, U>
where
    T: Component<Mutability = Mutable> + Clone,
    U: Component,
{
    pub(crate) dioxus_tx: Sender<T>,
    pub(crate) dioxus_rx: Receiver<T>,
    pub(crate) bevy_tx: Sender<T>,
    pub(crate) bevy_rx: Receiver<T>,
    singleton_marker: PhantomData<U>,
}

impl<T, U> RequestBevyComponentSingleton<T, U>
where
    T: Component<Mutability = Mutable> + Clone,
    U: Component,
{
    pub fn new() -> Self {
        let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<T>();
        let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<T>();

        Self {
            dioxus_tx,
            dioxus_rx,
            bevy_tx,
            bevy_rx,
            singleton_marker: PhantomData::default(),
        }
    }
}

impl<T, U> Command for RequestBevyComponentSingleton<T, U>
where
    T: Component<Mutability = Mutable> + Clone,
    U: Component,
{
    fn apply(self, world: &mut World) -> () {
        world.insert_resource(BevyTxChannel(self.bevy_tx));
        world.insert_resource(BevyRxChannel(self.bevy_rx));

        add_systems_through_world(world, Update, send_component_singleton::<T, U>);
        add_systems_through_world(world, Update, receive_component_update::<T, U>);
    }
}

fn send_component_singleton<T, U>(
    singleton: Query<(&T, &U), Changed<T>>,
    bevy_tx: ResMut<BevyTxChannel<T>>,
) where
    T: Component + Clone,
    U: Component,
{
    let Ok((value, _)) = singleton.single() else {
        return;
    };
    let _ = bevy_tx
        .0
        .send(value.clone())
        .inspect_err(|err| warn!("{:#}", err));
}

fn receive_component_update<T, U>(
    mut singleton: Query<&mut T, With<U>>,
    bevy_rx: ResMut<BevyRxChannel<T>>,
) where
    T: Component<Mutability = Mutable> + Clone,
    U: Component,
{
    let Ok(mut singleton) = singleton.single_mut() else {
        return;
    };
    let Ok(new_value) = bevy_rx.0.try_recv() else {
        return;
    };

    *singleton = new_value
}
