use std::{any::type_name, fmt::Display};

use async_std::task::sleep;
use bevy_dioxus_interop::{BevyCommandQueueTx, BoxAnyTypeMap, InfoRefershRateMS};
use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus_core::use_hook;
use dioxus_hooks::{try_use_context, use_context, use_future};
use dioxus_signals::{ReadableExt, Signal, SignalSubscriberDrop, SyncSignal, SyncStorage, UnsyncStorage, WritableExt, WriteLock};

use crate::{request_bevy_signal, resource::command::RequestBevyResource, traits::ErasedSubGenericResourcecMap, BevyFetchBackup, BevyRegistryKindErased, BevyValue, BevyValueNewType};

// fn request_resource_signal<T: Resource + Clone>(
//     command_queue_tx: Option<BevyCommandQueueTx>,
//     mut signal_registry: WriteLock<
//         '_,
//         ResourcesErased,
//         UnsyncStorage,
//         SignalSubscriberDrop<ResourcesErased, UnsyncStorage>,
//     >,
// ) -> SyncSignal<BevyRes<T>> {

//     if let Some(command_queue_tx) = command_queue_tx {
//         let mut commands = CommandQueue::default();

//         let command = RequestBevyResource::<T>::new();

//         let dioxus_rx = command.dioxus_rx.clone();
//         let dioxus_tx = command.dioxus_tx.clone();
//         commands.push(command);

//         let new_signal = SyncSignal::new_maybe_sync(BevyRes {
//             value: Err(BevyFetchBackup::Uninitialized),
//             resource_read: Some(dioxus_rx),
//             resource_write: Some(dioxus_tx),
//         });

//         signal_registry.insert(new_signal.clone());
//         let _ = command_queue_tx.0
//             .send(commands)
//             .inspect_err(|err| warn!("{:#}", err));

//         return new_signal;
//     } else {
//         let new_signal = SyncSignal::new_maybe_sync(BevyRes {
//             value: Err(BevyFetchBackup::Unknown),
//             resource_read: None,
//             resource_write: None,
//         });
//         new_signal
//     }


// }

/// Dioxus signals of Dioxus copies of Bevy resources.
#[derive(Clone, Default)]
pub struct ResourceSignals(Signal<ResourcesErased>);

/// requests a resource from bevy.
pub fn use_bevy_resource<T: Resource + Clone + Display>() -> SyncSignal<BevyValue<T>> {
    let refresh_rate = try_use_context::<InfoRefershRateMS>();

    let command_queue_tx = try_use_context::<BevyCommandQueueTx>();
    let mut signals_register = use_context::<ResourceSignals>();

    let signal = use_hook(|| {
        let mut map_erased = signals_register.0.write();

        let value = map_erased.get::<T>();
        let signal = if let Some(signal) = value {
            signal.clone()
        } else {
            request_bevy_signal(command_queue_tx, map_erased, RequestBevyResource::new())
            //request_resource_signal::<T>(command_queue_tx, map_erased.0)
        };
        signal
    });
    use_future(move || {
        let mut refresh_rate = refresh_rate.clone();
        async move {
            let mut signal: Signal<BevyValue<T>, SyncStorage> =
                signal.clone();
            let Some(reader) = signal.clone().write().reader.clone() else {
                return
            };
            let refresh_rate = refresh_rate.take().unwrap_or_default().0;
            loop {
                while let Ok(value) = reader.try_recv() {
                    let mut asset = signal.write();
                    asset.value = Ok(value)
                }
                sleep(std::time::Duration::from_millis(refresh_rate)).await;
            }
        }
    });
    signal
}

pub struct ResourceNewtype<T: Resource>(T);

impl<T: Resource> BevyValueNewType for ResourceNewtype<T> {

}
#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct ResourcesErased(BoxAnyTypeMap);

impl ErasedSubGenericResourcecMap for ResourcesErased {
    type Generic<T: Clone + Resource + Send + Sync + 'static> = SyncSignal<BevyValue<T>>;
}

impl<T: Resource + Clone> BevyRegistryKindErased<T> for ResourcesErased {
    type RequestKind = RequestBevyResource<T>;
} 
