use async_std::task::sleep;
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, world::CommandQueue};
use crossbeam_channel::{Receiver, Sender};
use dioxus::{core::use_hook, hooks::{use_context, use_future}, signals::{Signal, SignalSubscriberDrop, SyncSignal, UnsyncStorage, WritableExt, WriteLock}};
use std::fmt::{Debug, Display};

use crate::{dioxus_in_bevy_plugin::DioxusProps, traits::ErasedSubGenericResourcecMap, *};
pub enum InsertDefaultResource<T: Resource + Clone> {
    No,
    Yes(T),
}

/// Command to register dioxus bevy interop for a given resource.
pub(crate) struct RequestBevyResource<T: Resource + Clone> {
    default_resource: InsertDefaultResource<T>,

    pub(crate) dioxus_tx: Sender<T>,
    pub(crate) dioxus_rx: Receiver<T>,
    pub(crate) bevy_tx: Sender<T>,
    pub(crate) bevy_rx: Receiver<T>,
}

impl<T: Resource + Clone> RequestBevyResource<T> {
    pub fn new(default_resource: InsertDefaultResource<T>) -> Self {
        let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<T>();
        let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<T>();

        Self {
            default_resource,
            dioxus_tx,
            dioxus_rx,
            bevy_tx,
            bevy_rx,
        }
    }
}

impl<T: Resource + Clone> Command for RequestBevyResource<T> {
    fn apply(self, world: &mut World) -> () {
        world.insert_resource(BevyTxChannel(self.bevy_tx));
        world.insert_resource(BevyRxChannel(self.bevy_rx));

        add_systems_through_world(
            world,
            Update,
            send_resource_update::<T>.run_if(resource_changed::<T>),
        );
        add_systems_through_world(world, Update, receive_resource_update::<T>);
    }
}

fn send_resource_update<T: Resource + Clone>(
    resource: Res<T>,
    bevy_tx: ResMut<BevyTxChannel<T>>,
    // bevy_rx: ResMut<BevyRxChannel<T>>,
) {
    bevy_tx
        .0
        .send(resource.clone())
        .inspect_err(|err| warn!("could not send resource update due to {:#}", err));
}

fn receive_resource_update<T: Resource + Clone>(
    mut resource: ResMut<T>,
    bevy_rx: ResMut<BevyRxChannel<T>>,
    // bevy_rx: ResMut<BevyRxChannel<T>>,
) {
    let Ok(new_res) = bevy_rx.0.try_recv() else {
        return;
    };
    *resource = new_res;
}


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

    let command = RequestBevyResource::<T>::new(crate::resource_sync::InsertDefaultResource::No);

    let dioxus_rx = command.dioxus_rx.clone();
    let dioxus_tx = command.dioxus_tx.clone();
    commands.push(command);

    let new_signal = SyncSignal::new_maybe_sync(BevyRes {
        resource_read: None,
        resource_incoming: dioxus_rx,
        resource_write: dioxus_tx,
    });

    signal_registry.insert(new_signal.clone());
    props.command_queues_tx.send(commands);

    return new_signal;
}

/// Dioxus signals of Dioxus copies of Bevy resources.
#[derive(Clone, Default)]
pub struct ResourceSignals(Signal<ResourcesErased>);

/// requests a resource from bevy.
pub fn use_bevy_resource<T: Resource + Clone + Debug>() -> SyncSignal<BevyRes<T>> {
    let props = use_context::<DioxusProps>();

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
        let value = props.clone();
        async move {
            let mut signal: Signal<BevyRes<T>, dioxus::prelude::SyncStorage> = signal.clone();
            loop {
                sleep(std::time::Duration::from_millis(1000)).await;

                let mut resource = signal.write();
                warn!("attempting to receive resource");
                while let Ok(value) = resource.resource_incoming.try_recv() {
                    warn!("received value: {:#?}", value);
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

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct ResourcesErased(BoxAnyTypeMap);

impl ErasedSubGenericResourcecMap for ResourcesErased {
    type Generic<T: Clone + Resource + Send + Sync + 'static> = SyncSignal<BevyRes<T>>;
}


// #[derive(Clone, Default)]
// pub struct ResourceSignals(pub Signal<ResourceSignals>);

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

impl<T: Clone + Resource> BevyRes<T> {
    pub fn set_resource(&mut self, value: T) {
        self.resource_write
            .send(value.clone())
            .inspect_err(|err| warn!("could not update local resource signal due to {:#}", err));
        self.resource_read = Some(value)
    }
    pub fn read_resource(&self) -> &Option<T> {
        &self.resource_read
    }
}