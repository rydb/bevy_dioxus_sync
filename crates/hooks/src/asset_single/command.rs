use bevy_app::Update;
use bevy_asset::prelude::*;
use bevy_dioxus_interop::{add_systems_through_world, BevyRxChannel, BevyTxChannel};
use bevy_ecs::prelude::*;
use std::{any::type_name, marker::PhantomData, ops::Deref};

use bevy_log::warn;
use crossbeam_channel::{Receiver, Sender};

use crate::asset_single::AssetWithHandle;

/// Command to register dioxus bevy interop for a given resource.
pub(crate) struct RequestBevyWrappedAsset<T, U, V>
where
    T: Deref<Target = Handle<U>> + Component,
    U: Asset + Clone,
    V: Component,
{
    pub(crate) dioxus_tx: Sender<AssetWithHandle<T, U>>,
    pub(crate) dioxus_rx: Receiver<AssetWithHandle<T, U>>,
    pub(crate) bevy_tx: Sender<AssetWithHandle<T, U>>,
    pub(crate) bevy_rx: Receiver<AssetWithHandle<T, U>>,
    singleton_marker: PhantomData<V>,
}

impl<T, U, V> RequestBevyWrappedAsset<T, U, V>
where
    T: Deref<Target = Handle<U>> + Component,
    U: Asset + Clone,
    V: Component,
{
    pub fn new() -> Self {
        let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<AssetWithHandle<T, U>>();
        let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<AssetWithHandle<T, U>>();

        Self {
            dioxus_tx,
            dioxus_rx,
            bevy_tx,
            bevy_rx,
            singleton_marker: PhantomData::default(),
        }
    }
}

impl<T, U, V> Command for RequestBevyWrappedAsset<T, U, V>
where
    T: Deref<Target = Handle<U>> + Component,
    U: Asset + Clone,
    V: Component,
{
    fn apply(self, world: &mut World) -> () {
        world.insert_resource(BevyTxChannel(self.bevy_tx));
        world.insert_resource(BevyRxChannel(self.bevy_rx));

        add_systems_through_world(world, Update, send_asset_singleton::<T, U, V>);
        add_systems_through_world(world, Update, receive_asset_update::<T, U>);
    }
}

fn send_asset_singleton<T, U, V>(
    handle: Query<(Entity, &T, &V), Changed<T>>,
    bevy_tx: ResMut<BevyTxChannel<AssetWithHandle<T, U>>>,
    assets: ResMut<Assets<U>>,
) where
    T: Deref<Target = Handle<U>> + Component,
    U: Asset + Clone,
    V: Component,
{
    let Ok((_, handle, _)) = handle.single()
    //.inspect_err(|err| warn!("could not get singleton: {:#} ", err))
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
            new_type: PhantomData::default(),
        })
        .inspect_err(|err| warn!("could not send {:#}: {:#}", type_name::<T>(), err));
}

fn receive_asset_update<T, U>(
    bevy_rx: ResMut<BevyRxChannel<AssetWithHandle<T, U>>>,
    mut assets: ResMut<Assets<U>>,
) where
    T: Deref<Target = Handle<U>> + Component,
    U: Asset + Clone,
{
    let Ok(new_asset) = bevy_rx.0.try_recv() else {
        return;
    };

    assets.insert(&new_asset.handle, new_asset.asset);
}
