use std::{any::type_name, marker::PhantomData};

use async_std::task::sleep;
use bevy_dioxus_interop::{BevyCommandQueueTx, BoxAnyTypeMap, InfoRefershRateMS};
use bevy_ecs::{component::Mutable, prelude::*, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus_core::use_hook;
use dioxus_hooks::{use_context, use_future};
use dioxus_signals::{Signal, SignalSubscriberDrop, SyncSignal, UnsyncStorage, WritableExt, WriteLock};

use crate::{component_single::{command::RequestBevyComponentSingleton, BevyComponentSingleton}, traits::{ErasedSubGenericComponentSingletonMap}};

pub fn use_bevy_component_singleton<T, U>() -> SyncSignal<BevyComponentSingleton<T, U>>
where
    T: Component<Mutability = Mutable> + Clone,
    U: Component,
{
    //let props = use_context::<DioxusProps>();
    let refresh_rate = use_context::<InfoRefershRateMS>();

    let mut signals_register = use_context::<BevyComponentSignletonSignals>();
    let command_queue_tx = use_context::<BevyCommandQueueTx>();

    let signal = use_hook(|| {
        let mut map_erased = signals_register.0.write();

        let value = map_erased.get::<T, U>();
        let signal = if let Some(signal) = value {
            signal.clone()
        } else {
            request_component_channels(command_queue_tx, map_erased)
        };
        signal
    });

    use_future(move || {
        let refresh_rate = refresh_rate.clone();

        async move {
            let mut signal = signal.clone();
            loop {
                sleep(std::time::Duration::from_millis(refresh_rate.0)).await;

                let mut asset = signal.write();
                while let Ok(value) = asset.read.try_recv() {
                    asset.value = Some(value)
                }
            }
        }
    });
    signal
}

fn request_component_channels<T, U>(
    command_queue_tx: BevyCommandQueueTx,
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
    let mut commands = CommandQueue::default();

    let command = RequestBevyComponentSingleton::<T, U>::new();

    let dioxus_rx = command.dioxus_rx.clone();
    let dioxus_tx = command.dioxus_tx.clone();
    commands.push(command);

    let new_signal = SyncSignal::new_maybe_sync(BevyComponentSingleton {
        value: None,
        read: dioxus_rx,
        write: dioxus_tx,
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
}

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct ComponentsSingletonsErased(BoxAnyTypeMap);

impl ErasedSubGenericComponentSingletonMap for ComponentsSingletonsErased {
    type Generic<T: Component + Clone, U: Component> = SyncSignal<BevyComponentSingleton<T, U>>;
}

#[derive(Clone, Default)]
pub struct BevyComponentSignletonSignals(Signal<ComponentsSingletonsErased>);
