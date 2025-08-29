use std::{any::type_name, ops::Deref};

use async_std::task::sleep;
use bevy_asset::prelude::*;
use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus::{
    core::use_hook,
    hooks::{use_context, use_future},
    signals::{Signal, SignalSubscriberDrop, SyncSignal, UnsyncStorage, WritableExt, WriteLock},
};

use crate::{
    BoxAnyTypeMap,
    dioxus_in_bevy_plugin::DioxusProps,
    hooks::asset_single::{BevyWrappedAsset, command::RequestBevyWrappedAsset},
    traits::ErasedSubGenericAssetsMap,
    ui::InfoRefershRateMS,
};

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct BevyWrappedAssetsErased(BoxAnyTypeMap);

impl ErasedSubGenericAssetsMap for BevyWrappedAssetsErased {
    type Generic<
        T: Deref<Target = Handle<U>> + Component + Send + Sync + 'static,
        U: Asset + Clone,
    > = SyncSignal<BevyWrappedAsset<T, U>>;
}

#[derive(Clone, Default)]
pub struct BevyWrappedAssetsSignals(Signal<BevyWrappedAssetsErased>);

fn request_asset_channel<T, U, V>(
    props: DioxusProps,
    mut signal_registry: WriteLock<
        '_,
        BevyWrappedAssetsErased,
        UnsyncStorage,
        SignalSubscriberDrop<BevyWrappedAssetsErased, UnsyncStorage>,
    >,
) -> SyncSignal<BevyWrappedAsset<T, U>>
where
    T: Deref<Target = Handle<U>> + Component,
    U: Asset + Clone,
    V: Component,
{
    let mut commands = CommandQueue::default();

    let command = RequestBevyWrappedAsset::<T, U, V>::new();

    let dioxus_rx = command.dioxus_rx.clone();
    let dioxus_tx = command.dioxus_tx.clone();
    commands.push(command);

    let new_signal = SyncSignal::new_maybe_sync(BevyWrappedAsset {
        value: None,
        read: dioxus_rx,
        write: dioxus_tx,
    });

    signal_registry.insert(new_signal.clone());
    let _ = props.command_queues_tx.send(commands).inspect_err(|err| {
        warn!(
            "could not send command for {:#}: {:#}",
            type_name::<T>(),
            err
        )
    });

    return new_signal;
}

/// requests an asset from bevy.
pub fn use_bevy_asset_singleton<T, U, V>() -> SyncSignal<BevyWrappedAsset<T, U>>
where
    T: Deref<Target = Handle<U>> + Component,
    U: Asset + Clone,
    V: Component,
{
    let props = use_context::<DioxusProps>();
    let refresh_rate = use_context::<InfoRefershRateMS>();

    let mut signals_register = use_context::<BevyWrappedAssetsSignals>();

    let signal = use_hook(|| {
        let mut map_erased = signals_register.0.write();

        let value = map_erased.get::<T, U>();
        let signal = if let Some(signal) = value {
            signal.clone()
        } else {
            request_asset_channel::<T, U, V>(props.clone(), map_erased)
        };
        signal
    });

    use_future(move || {
        let refresh_rate = refresh_rate.clone();
        async move {
            let mut signal: Signal<BevyWrappedAsset<T, U>, dioxus::prelude::SyncStorage> =
                signal.clone();
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
