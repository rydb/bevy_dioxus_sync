use std::{any::type_name, marker::PhantomData};

use async_std::task::sleep;
use bevy_dioxus_interop::{BevyCommandQueueTx, BoxAnyTypeMap, InfoRefershRateMS};
use bevy_ecs::{component::Mutable, prelude::*, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus_core::use_hook;
use dioxus_hooks::{try_use_context, use_context, use_future};
use dioxus_signals::{Signal, SignalSubscriberDrop, SyncSignal, UnsyncStorage, WritableExt, WriteLock};

use crate::{component_single::{command::RequestBevyComponentSingleton, BevyComponentSingleton}, traits::{ErasedSubGenericComponentSingletonMap}};


pub fn use_bevy_component_singleton<T, U>() -> SyncSignal<BevyComponentSingleton<T, U>>
where
    T: Component<Mutability = Mutable> + Clone,
    U: Component,
{
    let refresh_rate = try_use_context::<InfoRefershRateMS>();

    let command_queue_tx = try_use_context::<BevyCommandQueueTx>();
    let mut signals_register = use_context::<BevyComponentSignletonSignals>();

    let signal = use_hook(|| {
        let mut map_erased = signals_register.0.write();

        let value = map_erased.get::<T, U>();
        let signal = if let Some(signal) = value {
            signal.clone()
        } else {
            request_component_signal::<T, U>(command_queue_tx, map_erased)
        };
        signal
    });

    use_future(move || {
        let mut refresh_rate = refresh_rate.clone();
        async move {
            let mut signal =
                signal.clone();

            if let Some(asset_reader) = &signal.clone().write().read {

                loop {
                    let mut asset = signal.write();
                    while let Ok(value) = asset_reader.try_recv() {
                        asset.value = Ok(value)
                    }

                    sleep(std::time::Duration::from_millis(refresh_rate.take().unwrap_or_default().0)).await;
                }
            // don't update asset if there is no asset/bevy connection to update the asset value with.
            } else {

            }
        }
    });
    signal
}

// pub fn use_bevy_component_singleton<T, U>() -> SyncSignal<BevyComponentSingleton<T, U>>
// where
//     T: Component<Mutability = Mutable> + Clone,
//     U: Component,
// {
//     //let props = use_context::<DioxusProps>();
//     let refresh_rate = try_use_context::<InfoRefershRateMS>();

//     let mut signals_register = use_context::<BevyComponentSignletonSignals>();
//     let command_queue_tx = try_use_context::<BevyCommandQueueTx>();

//     let signal = use_hook(|| {
//         let mut map_erased = signals_register.0.write();

//         let value = map_erased.get::<T, U>();
//         let signal = if let Some(signal) = value {
//             signal.clone()
//         } else {
//             request_component_signal(command_queue_tx, map_erased)
//         };
//         signal
//     });

//     use_future(move || {
//         let refresh_rate = refresh_rate.clone();

//         async move {
//             let mut signal = signal.clone();
//             loop {
//                 sleep(std::time::Duration::from_millis(refresh_rate.0)).await;

//                 let mut asset = signal.write();
//                 while let Ok(value) = asset.read.try_recv() {
//                     asset.value = Some(value)
//                 }
//             }
//         }
//     });
//     signal
// }

fn request_component_signal<T, U>(
    command_queue_tx: Option<BevyCommandQueueTx>,
    mut signal_registry: WriteLock<
        '_,
        ComponentsSingletonsErased,
        UnsyncStorage,
        SignalSubscriberDrop<ComponentsSingletonsErased, UnsyncStorage>,
    >,
) -> SyncSignal<BevyComponentSingleton<T, U>>
where
    T: Component<Mutability = Mutable> + Clone,
    U: Component,
{
    if let Some(command_queue_tx) = command_queue_tx {
        let mut commands = CommandQueue::default();

        let command = RequestBevyComponentSingleton::<T, U>::new();

        let dioxus_rx = command.dioxus_rx.clone();
        let dioxus_tx = command.dioxus_tx.clone();
        commands.push(command);

        let new_signal = SyncSignal::new_maybe_sync(BevyComponentSingleton {
            value: Err(crate::BevyFetchBackup::Uninitialized),
            read: Some(dioxus_rx),
            write: Some(dioxus_tx),
            _marker: PhantomData::default(),
        });

        signal_registry.insert(new_signal.clone());
        let _ = command_queue_tx.0.send(commands).inspect_err(|err| {
            warn!(
                "could not request component channel for {:#}: {:#}",
                type_name::<T>(),
                err
            )
        });

        return new_signal;
    } else {
        let new_signal = SyncSignal::new_maybe_sync(BevyComponentSingleton {
            value: Err(crate::BevyFetchBackup::Unknown),
            read: None,
            write: None,
            _marker: PhantomData::default(),
        });
        new_signal
    }
}

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct ComponentsSingletonsErased(BoxAnyTypeMap);

impl ErasedSubGenericComponentSingletonMap for ComponentsSingletonsErased {
    type Generic<T: Component + Clone, U: Component> = SyncSignal<BevyComponentSingleton<T, U>>;
}

#[derive(Clone, Default)]
pub struct BevyComponentSignletonSignals(Signal<ComponentsSingletonsErased>);
