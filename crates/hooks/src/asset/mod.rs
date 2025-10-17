use std::{any::{type_name, Any, TypeId}, collections::HashMap, marker::PhantomData, ops::Deref};

use bevy_asset::{io::AssetSourceId, Asset, AssetEvent, AssetId, Assets, Handle, UntypedAssetId};
use bevy_dioxus_interop::{add_systems_through_world, BevyRxChannel, BevyTxChannel};
use bevy_ecs::prelude::*;
use bevy_app::prelude::*;
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus_signals::{Signal, SyncSignal};

use crate::{use_bevy_value, BevyValue, BoxGenericTypeMap, DioxusTxrX, SignalsErasedMap};

pub type UntypedSendAsset = Box<dyn Any + Send + Sync>;

type UntypedAssetMap = HashMap<UntypedAssetId, UntypedSendAsset>;

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct BevyAssets(pub BoxGenericTypeMap<UntypedAssetId, UntypedAssetId>);

#[derive(Clone, Default, TransparentWrapper)]
#[repr(transparent)]
pub struct BevyAssetsRegistry(Signal<BevyAssets>);
#[derive(Clone)]
pub enum AssetRequestFilter<T, U> {
    // All(UntypedAssetMap),
    Singleton(PhantomData<T>, PhantomData<U>)
}

//// Requests bevy assets of type [`T`] within the given scope.
#[derive(Clone)]
pub struct RequestScopedAssetWithMarker<T, U, V> 
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone,
        V: Component,
{
    pub(crate) filter: AssetRequestFilter<V, U>,
    pub(crate) dioxus_tx: Sender<U>,
    pub(crate) dioxus_rx: Receiver<U>,
    pub(crate) bevy_tx: Sender<U>,
    pub(crate) bevy_rx: Receiver<U>,
    pub(crate) scope: PhantomData<T>
}

impl<T, U, V> Command for RequestScopedAssetWithMarker<T, U, V> 
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone,
        V: Component,
{
    fn apply(self, world: &mut World) -> () {
        world.insert_resource(BevyTxChannel(self.bevy_tx));
        world.insert_resource(BevyRxChannel(self.bevy_rx));


        add_systems_through_world(world, Update, send_asset_updates_singleton::<T, U, V>);
        add_systems_through_world(world, Update, receive_asset_update::<U>);
    }
}


fn receive_asset_update<T>(
    bevy_rx: ResMut<BevyRxChannel<T>>,
    mut assets: ResMut<Assets<T>>,
) where
    T: Asset + Clone,
{
    let Ok(new_asset) = bevy_rx.0.try_recv() else {
        return;
    };

    assets.insert(&new_asset.handle, new_asset.asset);
}

fn send_asset_updates_singleton<T, U, V>(
    handle: Query<(Entity, &T, &V), Changed<T>>,
    bevy_tx: ResMut<BevyTxChannel<BevyAsset<T>>,
    assets: ResMut<Assets<U>>,
) 
where
    T: Deref<Target = Handle<U>> + Component,
    U: Asset + Clone,
    V: Component,

{

    let Ok((e, handle, u)) = handle.single()
    .inspect_err(|err| warn!("could not get singleton: {:#} ", err))
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
        .send(asset.clone())
        .inspect_err(|err| warn!("could not send {:#}: {:#}", type_name::<U>(), err));
}

impl<T, U> Into<DioxusTxrX<U>> for RequestScopedAssetWithMarker<T, U> 
    where
        T: Component + Clone,
        U: Asset + Clone,
{
    fn into(self) -> DioxusTxrX<U> {
        DioxusTxrX { dioxus_tx: self.dioxus_tx, dioxus_rx: self.dioxus_rx }
    }
}


impl<T, U> Default for RequestScopedAssetWithMarker<T, U> 
    where
        T: Component + Clone,
        U: Asset + Clone,
{
    fn default() -> Self {
        let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<U>();
        let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<U>();
        Self {
            bevy_tx,
            bevy_rx,
            dioxus_tx,
            dioxus_rx,
            filter: AssetRequestFilter::Singleton(PhantomData::<T>::default(), PhantomData::<U>::default())
        }
    }
}

pub type BevyAsset<T> = BevyValue<T, UntypedAssetId>;

impl SignalsErasedMap for BevyAssets {
    type Value<T: Clone + Send + Sync + 'static> = SyncSignal<BevyAsset<T>>;
    type Index = UntypedAssetId;
}

// pub fn request_bevy_asset_id<T, U>(filter: AssetRequestFilter::<T, U>) -> UntypedAssetId{

// }

/// use a bevy component thats a newtype handle around an asset. Looks for a singleton with the given marker component.
pub fn use_bevy_component_asset_single<T, U, V>() -> SyncSignal<BevyAsset<U>>
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone,
        V: Component + Clone,
    {
    // let asset_id = request_bevy_asset_id::<T, U>(AssetRequestFilter::Singleton(PhantomData::<T>::default(),PhantomData::<U>::default()));
    use_bevy_value::<U, BevyAssets, UntypedAssetMap, RequestScopedAssetWithMarker<T, V, U>, UntypedAssetId>(TypeId::of::<U>())

}