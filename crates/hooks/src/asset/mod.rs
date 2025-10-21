use std::{any::{type_name, Any, TypeId}, collections::HashMap, marker::PhantomData, ops::Deref};

use bevy_asset::{io::AssetSourceId, Asset, AssetEvent, AssetId, Assets, Handle, UntypedAssetId};
use bevy_dioxus_interop::{add_systems_through_world, BevyDioxusIO, BevyRxChannel, BevyTxChannel, InfoPacket};
use bevy_ecs::prelude::*;
use bevy_app::prelude::*;
use bevy_log::warn;
use bevy_time::Time;
use bevy_utils::default;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus_signals::{Signal, SyncSignal};

use crate::{resource::command::RequestBevyResource, use_bevy_value, AdditionalInfo, BevyValue, BoxGenericTypeMap, Channels, DioxusTxRx, RequestA, SignalsErasedMap};

pub type UntypedSendAsset = Box<dyn Any + Send + Sync>;

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct BevyAssets(pub BoxGenericTypeMap<UntypedAssetId>);



#[derive(Clone, Default, TransparentWrapper)]
#[repr(transparent)]
pub struct BevyAssetsRegistry(Signal<BevyAssets>);


#[derive(Clone)]
pub enum AssetRequestFilter<T, U> {
    // All(UntypedAssetMap),
    Singleton(PhantomData<T>, PhantomData<U>)
}

#[derive(TransparentWrapper, Clone)]
#[repr(transparent)]
#[transparent(BevyDioxusIO<AssetValue<U>, AssetInfoIndex, AssetAdditionalInfo>)]
pub struct RequestScopedAssetWithMarker<
    T: Deref<Target = Handle<U>> + Component + Clone, 
    U: Asset + Clone, 
    V: Component + Clone
> 
{
    pub(crate) channels: BevyDioxusIO<AssetValue<U>, AssetInfoIndex, AssetAdditionalInfo>,
    _handle_check_phantom: PhantomData<V>,
    _marker_filter_phantom: PhantomData<T>
}

impl<T, U, V> Command for RequestScopedAssetWithMarker<T, U, V> 
    where
        T: Deref<Target = Handle<U>> + Component + Clone, 
        U: Asset + Clone, 
        V: Component + Clone
{
    fn apply(self, world: &mut World) -> () {
        world.insert_resource(BevyTxChannel(self.channels.bevy_tx));
        world.insert_resource(BevyRxChannel(self.channels.bevy_rx));


        add_systems_through_world(world, Update, send_asset_updates_singleton::<T, U, V>);
        add_systems_through_world(world, Update, receive_asset_updates::<U>);
    }
}

type AssetInfoIndex = UntypedAssetId;
type AssetValue<T> = T;
type AssetAdditionalInfo = UntypedAssetId;
type AssetInfoPacket<T> = InfoPacket<AssetValue<T>, AssetInfoIndex, AssetAdditionalInfo>;

fn receive_asset_updates<U>(
    bevy_rx: ResMut<BevyRxChannel<AssetInfoPacket<U>>>,
    mut assets: ResMut<Assets<U>>,
) 
    where
        U: Asset + Clone
{
    let Ok(packet) = bevy_rx.0.try_recv() else {
        return;
    };
    let Some(index) = packet.index else {
        warn!("asset update received, but index not given with packet update?");
        return;
    };
    let _ = assets.insert(index.typed(), packet.update).inspect_err(|err| warn!("{err}"));

}

fn send_asset_updates_singleton<T, U, V>(
    handle: Query<(Entity, &T, &V), Changed<T>>,
    bevy_tx: ResMut<BevyTxChannel<AssetInfoPacket<U>>>,
    assets: ResMut<Assets<U>>,
) 
    where
        T: Deref<Target = Handle<U>> + Component + Clone, 
        U: Asset + Clone, 
        V: Component + Clone
{

    let Ok((_e, handle, _u)) = handle.single()
    //.inspect_err(|err| warn!("could not get singleton: {:#} ", err))
    else {
        return;
    };

    let handle = (**handle).clone();
    let Some(asset) = assets.get(&handle) else {
        warn!("could not get asset from {:#?}", handle);
        return;
    };
    warn!("sending asset update..");
    let _ = bevy_tx
        .0
        .send(
            InfoPacket { update: asset.clone(), index: Some(handle.id().untyped()), additional_info: Some(handle.id().untyped()) }
        )
        .inspect_err(|err| warn!("could not send {:#}: {:#}", type_name::<U>(), err));
}


impl<T, U, V> Default for RequestScopedAssetWithMarker<T, U, V> 
    where
        T: Deref<Target = Handle<U>> + Component + Clone, 
        U: Asset + Clone, 
        V: Component + Clone
    {
        fn default() -> Self {
        Self {
            channels: BevyDioxusIO::default(),
            _handle_check_phantom: PhantomData,
            _marker_filter_phantom: PhantomData,
            
        }
    }
}

pub type BevyAsset<T> = BevyValue<T, UntypedAssetId, UntypedAssetId>;

impl SignalsErasedMap for BevyAssets {
    type Index = UntypedAssetId;
    type AdditionalInfo = UntypedAssetId;
}

/// use a bevy component thats a newtype handle around an asset. Looks for a singleton with the given marker component.
pub fn use_bevy_component_asset_single<T, U, V>() -> SyncSignal<BevyValue<U, UntypedAssetId, UntypedAssetId>>
    where
        T: Deref<Target = Handle<U>> + Component + Clone, 
        U: Asset + Clone, 
        V: Component + Clone,
    {
        use_bevy_value::<U, BevyAssetsRegistry, BevyAssets, RequestScopedAssetWithMarker<T, U, V>>(None)
}