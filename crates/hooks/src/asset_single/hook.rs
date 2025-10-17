use std::{any::type_name, ops::Deref};

use async_std::task::sleep;
use bevy_asset::prelude::*;
use bevy_dioxus_interop::{BevyCommandQueueTx, BoxAnyTypeMap, InfoRefershRateMS};
use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus_core::use_hook;
use dioxus_hooks::{try_use_context, use_context, use_future};
use dioxus_signals::{Signal, SignalSubscriberDrop, SyncSignal, SyncStorage, UnsyncStorage, WritableExt, WriteLock};

use crate::{asset_single::{command::RequestBevyAsset, BevyFetchBackup, BevyWrappedAsset}, traits::ErasedSubGenericAssetsMap, use_bevy_value, BevyRegistryKindErased, BevyValue, SyncSignalErasedMap};

// #[derive(TransparentWrapper, Default)]
// #[repr(transparent)]
// pub struct BevyAssetsErased(BoxAnyTypeMap);

// impl ErasedSubGenericAssetsMap for BevyAssetsErased {
//     type Generic<
//         T: Deref<Target = Handle<U>> + Component + Send + Sync + 'static,
//         U: Asset + Clone,
//     > = SyncSignal<BevyWrappedAsset<T, U>>;
// }



// fn request_asset_signal<T, U, V>(
//     command_queue_tx: Option<BevyCommandQueueTx>,
//     mut signal_registry: WriteLock<
//         '_,
//         BevyAssetsErased,
//         UnsyncStorage,
//         SignalSubscriberDrop<BevyAssetsErased, UnsyncStorage>,
//     >,
// ) -> SyncSignal<BevyWrappedAsset<T, U>>
// where
//     T: Deref<Target = Handle<U>> + Component,
//     U: Asset + Clone,
//     V: Component,
// {
//     if let Some(command_queue_tx) = command_queue_tx {
//         let mut commands = CommandQueue::default();

//         let command = RequestBevyAsset::<T, U, V>::new();

//         let dioxus_rx = command.dioxus_rx.clone();
//         let dioxus_tx = command.dioxus_tx.clone();
//         commands.push(command);

//         let new_signal = SyncSignal::new_maybe_sync(BevyWrappedAsset {
//             value: Err(BevyFetchBackup::Uninitialized),
//             read: Some(dioxus_rx),
//             write: Some(dioxus_tx),
//         });

//         signal_registry.insert(new_signal.clone());
//         let _ = command_queue_tx.0.send(commands).inspect_err(|err| {
//             warn!(
//                 "could not send command for {:#}: {:#}",
//                 type_name::<T>(),
//                 err
//             )
//         });
//         return new_signal
//     } else {
//         let new_signal = SyncSignal::new_maybe_sync(BevyWrappedAsset {
//             value: Err(BevyFetchBackup::Unknown),
//             read: None,
//             write: None,
//         });
//         return new_signal
//     }
// }


// /// Loresm ipsum asset that prints lorem ipsum in the absensce of an asset.
// pub struct LoremIpsumAsset<T, U> {
//     _a: PhantomData<T>,
//     _b: PhantomData<U>,
// }



#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct BevyAssetsErased(SyncSignalErasedMap);

impl<T, U, V> BevyRegistryKindErased<T> for BevyAssetsErased 
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone,
        V: Component,
{
    type RequestKind = RequestBevyAsset<T, U, V>;
} 


#[derive(Clone, Default, TransparentWrapper)]
#[repr(transparent)]
pub struct BevyAssetsSignals(Signal<BevyAssetsErased>);


pub fn use_bevy_asset_singleton<T, U, V>() -> SyncSignal<BevyValue<U>>
where
    T: Deref<Target = Handle<U>> + Component,
    U: Asset + Clone,
    V: Component,
{
    use_bevy_value::<U, BevyAssetsSignals, BevyAssetsErased>()
}   


// pub type BevyAsset<T, U> = BevyWrappedAsset<T, U>;

// pub fn use_bevy_asset_singleton<T, U, V>() -> SyncSignal<BevyAsset<T, U>>
// where
//     T: Deref<Target = Handle<U>> + Component,
//     U: Asset + Clone,
//     V: Component,
// {
//     let refresh_rate = try_use_context::<InfoRefershRateMS>();

//     let command_queue_tx = try_use_context::<BevyCommandQueueTx>();
//     let mut signals_register = use_context::<BevyAssetsSignals>();

//     let signal = use_hook(|| {
//         let mut map_erased = signals_register.0.write();

//         let value = map_erased.get::<T, U>();
//         let signal = if let Some(signal) = value {
//             signal.clone()
//         } else {
//             request_asset_signal::<T, U, V>(command_queue_tx, map_erased)
//         };
//         signal
//     });

//     use_future(move || {
//         let mut refresh_rate = refresh_rate.clone();
//         async move {
//             let mut signal: Signal<BevyWrappedAsset<T, U>, SyncStorage> =
//                 signal.clone();

//             if let Some(asset_reader) = &signal.clone().write().read {

//                 loop {
//                     let mut asset = signal.write();
//                     while let Ok(value) = asset_reader.try_recv() {
//                         asset.value = Ok(value)
//                     }

//                     sleep(std::time::Duration::from_millis(refresh_rate.take().unwrap_or_default().0)).await;
//                 }
//             // don't update asset if there is no asset/bevy connection to update the asset value with.
//             } else {

//             }
//         }
//     });
//     signal
// }
