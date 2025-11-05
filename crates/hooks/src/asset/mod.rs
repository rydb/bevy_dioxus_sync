use std::{
    any::{Any, type_name},
    marker::PhantomData,
    ops::Deref,
};

use bevy_app::prelude::*;
use bevy_asset::{Asset, Assets, Handle, UntypedAssetId};
use bevy_dioxus_interop::{
    BevyDioxusIO, BevyRxChannel, BevyTxChannel, InfoPacket, InfoUpdate, StatusUpdate,
    add_systems_through_world,
};
use bevy_ecs::prelude::*;
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus_signals::{Signal, SyncSignal};
use std::fmt::Debug;

use crate::{BevyValue, BoxGenericTypeMap, SignalsErasedMap, use_bevy_value};

pub type UntypedSendAsset = Box<dyn Any + Send + Sync>;

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct BevyAssets(pub BoxGenericTypeMap<UntypedAssetId>);

#[derive(Clone, Default, TransparentWrapper)]
#[repr(transparent)]
pub struct BevyAssetsRegistry(Signal<BevyAssets>);

#[derive(Clone)]
pub enum AssetRequestFilter<T, U> {
    //TODO: Add more here
    Singleton(PhantomData<T>, PhantomData<U>),
}

#[derive(TransparentWrapper)]
#[repr(transparent)]
#[transparent(BevyDioxusIO<AssetValue<U>, AssetInfoIndex, AssetAdditionalInfo>)]
pub(crate) struct RequestScopedAssetWithMarker<
    T: Deref<Target = Handle<U>> + Component + Clone,
    U: Asset + Clone,
    V: Component + Clone,
> {
    pub(crate) channels: BevyDioxusIO<AssetValue<U>, AssetInfoIndex, AssetAdditionalInfo>,
    _handle_check_phantom: PhantomData<V>,
    _marker_filter_phantom: PhantomData<T>,
}

impl<T, U, V> Command for RequestScopedAssetWithMarker<T, U, V>
where
    T: Deref<Target = Handle<U>> + Component + Clone,
    U: Debug + Asset + Clone,
    V: Component + Clone,
{
    fn apply(self, world: &mut World) -> () {
        world.insert_resource(BevyTxChannel(self.channels.bevy_tx));
        world.insert_resource(BevyRxChannel(self.channels.bevy_rx));

        add_systems_through_world(world, Update, send_asset_updates_singleton::<T, U, V>);
        add_systems_through_world(world, Update, receive_asset_updates_singleton::<T, U, V>);
    }
}

type AssetInfoIndex = UntypedAssetId;
type AssetValue<T> = T;
type AssetAdditionalInfo = UntypedAssetId;

/// info send to/from bevy on asset data
type AssetInfoPacket<T> = InfoPacket<AssetValue<T>, AssetInfoIndex, AssetAdditionalInfo>;

fn receive_asset_updates_singleton<T, U, V>(
    handle: Query<(Entity, &T, &V)>,

    bevy_rx: ResMut<BevyRxChannel<AssetInfoPacket<U>>>,
    bevy_tx: ResMut<BevyTxChannel<AssetInfoPacket<U>>>,

    mut assets: ResMut<Assets<U>>,
) where
    T: Deref<Target = Handle<U>> + Component + Clone,
    U: Debug + Asset + Clone,
    V: Component + Clone,
{
    while let Ok(packet) = bevy_rx.0.try_recv().inspect_err(|err| match err {
        crossbeam_channel::TryRecvError::Empty => {}
        crossbeam_channel::TryRecvError::Disconnected => {
            warn!("could not receive asset: {:#}", err)
        }
    }) {
        match packet {
            InfoPacket::Update(info_update) => {
                let Some(index) = info_update.index else {
                    warn!("asset update received, but index not given with packet update?");
                    return;
                };
                let _ = assets
                    .insert(index.typed(), info_update.update)
                    .inspect_err(|err| warn!("{err}"));
            }
            InfoPacket::Request(status_update) => match status_update {
                StatusUpdate::RequestRefresh => {
                    let Ok((_e, handle, _u)) = handle.single().inspect_err(|err| match err {
                        bevy_ecs::query::QuerySingleError::MultipleEntities(_debug_name) => warn!(
                            "Asset for {:#} with marker {:#} no unique",
                            type_name::<T>(),
                            type_name::<V>()
                        ),
                        bevy_ecs::query::QuerySingleError::NoEntities(_debug_name) => {}
                    }) else {
                        return;
                    };
                    let handle = (**handle).clone();
                    let Some(asset) = assets.get(&handle) else {
                        warn!("could not get asset from {:#?}", handle);
                        return;
                    };
                    let packet = InfoUpdate {
                        update: asset.clone(),
                        index: Some(handle.id().untyped()),
                        additional_info: Some(handle.id().untyped()),
                    };
                    let _ = bevy_tx
                        .0
                        .send(InfoPacket::Update(packet))
                        .inspect_err(|err| {
                            warn!("could not send {:#}: {:#}", type_name::<U>(), err)
                        });
                }
            },
        }
    }
}

fn send_asset_updates_singleton<T, U, V>(
    handle: Query<(Entity, &T, &V), Or<(Changed<T>, Added<T>)>>,
    bevy_tx: ResMut<BevyTxChannel<AssetInfoPacket<U>>>,
    assets: ResMut<Assets<U>>,
) where
    T: Deref<Target = Handle<U>> + Component + Clone,
    U: Debug + Asset + Clone,
    V: Component + Clone,
{
    let Ok((_e, handle, _u)) = handle.single().inspect_err(|err| match err {
        bevy_ecs::query::QuerySingleError::MultipleEntities(_debug_name) => warn!(
            "Asset for {:#} with marker {:#} no unique",
            type_name::<T>(),
            type_name::<V>()
        ),
        bevy_ecs::query::QuerySingleError::NoEntities(_debug_name) => {}
    }) else {
        return;
    };

    let handle = (**handle).clone();
    let Some(asset) = assets.get(&handle) else {
        warn!("could not get asset from {:#?}", handle);
        return;
    };
    let packet = InfoUpdate {
        update: asset.clone(),
        index: Some(handle.id().untyped()),
        additional_info: Some(handle.id().untyped()),
    };
    let _ = bevy_tx
        .0
        .send(InfoPacket::Update(packet))
        .inspect_err(|err| warn!("could not send {:#}: {:#}", type_name::<U>(), err));
}

impl<T, U, V> Default for RequestScopedAssetWithMarker<T, U, V>
where
    T: Deref<Target = Handle<U>> + Component + Clone,
    U: Asset + Clone,
    V: Component + Clone,
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

/// interface with a singular bevy asset, [`U`]. Selects the asset based on its handle newtype, [`T`] and a marker component, [`V`].
pub fn use_bevy_component_asset_single<T, U, V>()
-> SyncSignal<BevyValue<U, UntypedAssetId, UntypedAssetId>>
where
    T: Deref<Target = Handle<U>> + Component + Clone,
    U: Debug + Asset + Clone,
    V: Component + Clone,
{
    use_bevy_value::<U, BevyAssetsRegistry, BevyAssets, RequestScopedAssetWithMarker<T, U, V>>(None)
}
