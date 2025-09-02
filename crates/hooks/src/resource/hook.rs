use std::fmt::Display;

use async_std::task::sleep;
use bevy_ecs::world::CommandQueue;
use dioxus::{
    core::use_hook,
    hooks::{use_context, use_future},
    signals::{Signal, SignalSubscriberDrop, SyncSignal, UnsyncStorage, WritableExt, WriteLock},
};

use crate::{
    plugins::DioxusProps, resource_sync::command::RequestBevyResource,
    traits::ErasedSubGenericResourcecMap, ui::InfoRefershRateMS, *,
};

fn request_resource_channel<T: Resource + Clone>(
    props: DioxusProps,
    mut signal_registry: WriteLock<
        '_,
        ResourcesErased,
        UnsyncStorage,
        SignalSubscriberDrop<ResourcesErased, UnsyncStorage>,
    >,
) -> SyncSignal<BevyRes<T>> {
    let mut commands = CommandQueue::default();

    let command = RequestBevyResource::<T>::new();

    let dioxus_rx = command.dioxus_rx.clone();
    let dioxus_tx = command.dioxus_tx.clone();
    commands.push(command);

    let new_signal = SyncSignal::new_maybe_sync(BevyRes {
        resource_read: None,
        resource_incoming: dioxus_rx,
        resource_write: dioxus_tx,
    });

    signal_registry.insert(new_signal.clone());
    let _ = props
        .command_queues_tx
        .send(commands)
        .inspect_err(|err| warn!("{:#}", err));

    return new_signal;
}

/// Dioxus signals of Dioxus copies of Bevy resources.
#[derive(Clone, Default)]
pub struct ResourceSignals(Signal<ResourcesErased>);

/// requests a resource from bevy.
pub fn use_bevy_resource<T: Resource + Clone + Display>() -> SyncSignal<BevyRes<T>> {
    let props = use_context::<DioxusProps>();
    let refresh_rate = use_context::<InfoRefershRateMS>();

    let mut resource_signals = use_context::<ResourceSignals>();

    let signal = use_hook(|| {
        let mut map_erased = resource_signals.0.write();

        let value = map_erased.get::<T>();
        let signal = if let Some(signal) = value {
            signal.clone()
        } else {
            request_resource_channel(props.clone(), map_erased)
        };
        signal
    });

    use_future(move || {
        // let value = props.clone();
        let refresh_rate = refresh_rate.clone();
        async move {
            let mut signal: Signal<BevyRes<T>, dioxus::prelude::SyncStorage> = signal.clone();
            loop {
                sleep(std::time::Duration::from_millis(refresh_rate.0)).await;

                let mut resource = signal.write();
                // warn!("attempting to receive resource");
                while let Ok(value) = resource.resource_incoming.try_recv() {
                    // warn!("received value: {:#?}", value);
                    resource.resource_read = Some(value)
                }
            }
        }
    });
    signal
}

pub struct BevyRes<T: Clone + Resource> {
    pub(crate) resource_write: Sender<T>,
    pub(crate) resource_incoming: Receiver<T>,
    //receiver: Receiver<T>,
    pub(crate) resource_read: Option<T>,
}

impl<T: Clone + Resource> BevyRes<T> {
    pub fn set_resource(&self, value: T) {
        let _ = self
            .resource_write
            .send(value.clone())
            .inspect_err(|err| warn!("could not update local resource signal due to {:#}", err));
    }
    pub fn read_resource(&self) -> &Option<T> {
        &self.resource_read
    }
}

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct ResourcesErased(BoxAnyTypeMap);

impl ErasedSubGenericResourcecMap for ResourcesErased {
    type Generic<T: Clone + Resource + Send + Sync + 'static> = SyncSignal<BevyRes<T>>;
}

impl<T: Clone + Resource + Display> Display for BevyRes<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.read_resource()
                .clone()
                .map(|n| format!("{}", n))
                .unwrap_or("???".to_string())
        )
    }
}
