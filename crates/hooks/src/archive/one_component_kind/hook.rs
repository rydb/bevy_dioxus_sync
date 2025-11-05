use std::{any::type_name, collections::HashMap};

use async_std::task::sleep;
use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;

use crate::{
    BoxAnyTypeMap,
    dioxus_in_bevy_plugin::DioxusProps,
    hooks::one_component_kind::{BevyQueryComponents, command::RequestBevyComponents},
    traits::ErasedSubGenericComponentsMap,
};

pub fn use_bevy_component_query<T: Component + Clone>() -> SyncSignal<BevyQueryComponents<T>> {
    let props = use_context::<DioxusProps>();

    let mut signals_register = use_context::<BevyComponentsSignals>();

    let signal = use_hook(|| {
        let mut map_erased = signals_register.0.write();

        let value = map_erased.get::<T>();
        let signal = if let Some(signal) = value {
            signal.clone()
        } else {
            request_component_channels(props.clone(), map_erased)
        };
        signal
    });

    use_future(move || {
        let value = props.clone();
        async move {
            let mut signal = signal.clone();
            loop {
                sleep(std::time::Duration::from_millis(1000)).await;

                let mut copies = signal.write();
                while let Ok(value) = copies.query_read.try_recv() {
                    // warn!("received entity component map");
                    copies
                        .components
                        .retain(|key, n| value.remove.contains(key) == false);
                    copies.components.extend(value.add);
                }
            }
        }
    });
    signal
}

fn request_component_channels<T: Component + Clone>(
    props: DioxusProps,
    mut signal_registry: WriteLock<
        '_,
        ComponentsErased,
        UnsyncStorage,
        SignalSubscriberDrop<ComponentsErased, UnsyncStorage>,
    >,
) -> SyncSignal<BevyQueryComponents<T>> {
    let mut commands = CommandQueue::default();

    let command = RequestBevyComponents::<T>::new();

    let dioxus_rx = command.io.dioxus_rx.clone();
    let dioxus_tx = command.io.dioxus_tx.clone();
    commands.push(command);

    let new_signal = SyncSignal::new_maybe_sync(BevyQueryComponents {
        components: HashMap::default(),
        query_read: dioxus_rx,
        query_write: dioxus_tx,
    });

    signal_registry.insert(new_signal.clone());
    let _ = props.command_queues_tx.send(commands).inspect_err(|err| {
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
pub struct ComponentsErased(BoxAnyTypeMap);

impl ErasedSubGenericComponentsMap for ComponentsErased {
    type Generic<T: Clone + Component + Send + Sync + 'static> = SyncSignal<BevyQueryComponents<T>>;
}

#[derive(Clone, Default)]
pub struct BevyComponentsSignals(Signal<ComponentsErased>);
