use bevy_app::Update;
use bevy_asset::prelude::*;
use bevy_dioxus_interop::{add_systems_through_world, BevyRxChannel, BevyTxChannel};
use bevy_ecs::prelude::*;
use std::{any::type_name, marker::PhantomData, ops::Deref};

use bevy_log::warn;
use crossbeam_channel::{Receiver, Sender};

use crate::{asset_single::AssetWithHandle, DioxusTxrX};

/// Command to register dioxus bevy interop for a given resource.
#[derive(Clone)]
pub(crate) struct RequestBevyAssetSingleton<T, U>
where
    T: Asset + Clone,
{
    pub(crate) dioxus_tx: Sender<AssetWithHandle<T>>,
    pub(crate) dioxus_rx: Receiver<AssetWithHandle<T>>,
    pub(crate) bevy_tx: Sender<AssetWithHandle<T>>,
    pub(crate) bevy_rx: Receiver<AssetWithHandle<T>>,
    singleton_marker: PhantomData<U>,
}

impl<T> Into<DioxusTxrX<T>> for RequestBevyAssetSingleton<T, U> {
    fn into(self) -> DioxusTxrX<T> {
        DioxusTxrX { dioxus_tx: self.dioxus_tx, dioxus_rx: self.dioxus_rx }
    }
}

impl<T, U, V> Default for RequestBevyAsset<T, U, V> 
where
    U: Asset + Clone,
    V: Component,
{
    fn default() -> Self {
        let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<AssetWithHandle<U>>();
        let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<AssetWithHandle<U>>();
        Self {
            bevy_tx,
            bevy_rx,
            dioxus_tx,
            dioxus_rx,
            singleton_marker: PhantomData::default()
        }
    }
}

// impl<T, U, V> RequestBevyAsset<T, U, V>
// where
//     T: Deref<Target = Handle<U>> + Component,
//     U: Asset + Clone,
//     V: Component,
// {
//     pub fn new() -> Self {
//         let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<AssetWithHandle<T, U>>();
//         let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<AssetWithHandle<T, U>>();

//         Self {
//             dioxus_tx,
//             dioxus_rx,
//             bevy_tx,
//             bevy_rx,
//             singleton_marker: PhantomData::default(),
//         }
//     }
// }

impl<U, V> Command for RequestBevyAssetSingleton<U, V>
where
    U: Asset + Clone,
    V: Component,
{
    fn apply(self, world: &mut World) -> () {
        world.insert_resource(BevyTxChannel(self.bevy_tx));
        world.insert_resource(BevyRxChannel(self.bevy_rx));

        add_systems_through_world(world, Update, send_asset_singleton::<U, V>);
        add_systems_through_world(world, Update, receive_asset_update::<U>);
    }
}

fn send_asset_singleton<U, V>(

    // handle: Query<(Entity, &V), Changed<V>>,
    mut messages: MessageReader<AssetEvent<U>>,
    bevy_tx: ResMut<BevyTxChannel<AssetWithHandle<U>>>,
    assets: ResMut<Assets<U>>,
) where
    U: Asset + Clone,
    V: Component,
{

    let Ok((_, handle, _)) = handle.single()
    else {
        return;
    };

    let handle = (**handle).clone();
    let Some(asset) = assets.get(&handle) else {
        warn!("could not get asset from {:#?}", handle);
        return;
    };

    let _ = bevy_tx
        .0
        .send(AssetWithHandle {
            asset: asset.clone(),
            handle: handle,
        })
        .inspect_err(|err| warn!("could not send {:#}: {:#}", type_name::<T>(), err));
}

fn receive_asset_update<U>(
    bevy_rx: ResMut<BevyRxChannel<AssetWithHandle<U>>>,
    mut assets: ResMut<Assets<U>>,
) where
    U: Asset + Clone,
{
    let Ok(new_asset) = bevy_rx.0.try_recv() else {
        return;
    };

    assets.insert(&new_asset.handle, new_asset.asset);
}
