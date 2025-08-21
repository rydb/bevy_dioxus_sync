use std::{any::{type_name, TypeId}, fmt::Display};

use async_std::task::sleep;
use bevy_ecs::{entity::Entity, world::{CommandQueue, World}};
use bevy_log::warn;
use bevy_platform::collections::HashMap;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus::prelude::*;
use bevy_utils::default;
use crate::{dioxus_in_bevy_plugin::{DioxusCommandQueueRx, DioxusProps}, resource_sync::RequestBevyResource, systems::PanelUpdateKind, traits::ErasedSubGeneriResourcecMap, ArcAnytypeMap, BoxAnyTypeMap, DioxusPanel, ErasedSubGenericMap};
use bevy_ecs::prelude::Resource;
use std::fmt::Debug;

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct SignalRegistry(BoxAnyTypeMap);

impl ErasedSubGeneriResourcecMap for SignalRegistry {
    type Generic<T: Clone + Resource + Send + Sync + 'static> = SyncSignal<DioxusRes<T>>;
}

/// Dioxus signals of Dioxus copies of Bevy resources.
#[derive(Clone, Default)]
pub struct ResourceSignalRegistry(Signal<SignalRegistry>);

#[derive(Clone, Default)]
pub struct DioxusPanels(pub Signal<HashMap<Entity, DioxusPanel>>);

#[derive(Clone, Default)]
pub struct ResourceSignals(pub Signal<ResourceSignalRegistry>);

pub struct DioxusRes<T: Clone + Resource> {
    pub(crate) resource_write: Sender<T>,
    pub(crate) resource_incoming: Receiver<T>,
    //receiver: Receiver<T>,
    pub(crate) resource_read: Option<T>
}

impl<T: Clone + Resource + Display> Display for DioxusRes<T> 
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.read_resource().clone().map(|n| format!("{}", n)).unwrap_or("???".to_string()))
    }
}


impl<T: Clone + Resource> DioxusRes<T> {
    pub fn set_resource(&mut self, value: T) {
        self.resource_write.send(value.clone()).inspect_err(|err| warn!("could not update local resource signal due to {:#}", err));
        self.resource_read = Some(value)
    }
    pub fn read_resource(&self) -> &Option<T> {
       &self.resource_read
    }
}

fn request_resource_channel<T:Resource + Clone>(props: DioxusProps, mut signal_registry: WriteLock<'_, SignalRegistry, UnsyncStorage, SignalSubscriberDrop<SignalRegistry, UnsyncStorage>>) -> SyncSignal<DioxusRes<T>> {
    let mut commands = CommandQueue::default();

    let command = RequestBevyResource::<T>::new(crate::resource_sync::InsertDefaultResource::No);
    
    let dioxus_rx = command.dioxus_rx.clone();
    let dioxus_tx = command.dioxus_tx.clone();
    commands.push(command);

    let new_signal = SyncSignal::new_maybe_sync(DioxusRes {
        resource_read: None,
        resource_incoming: dioxus_rx,
        resource_write: dioxus_tx
    });

    // let mut dioxus_resource_copies = resource_signals.0.write();
    //resource_signal_write.0.write().insert(new_signal.clone());
    signal_registry.insert(new_signal.clone());
    props.command_queues_tx.send(commands);

    return new_signal
}

/// requests a resource from bevy. 
pub fn use_bevy_resource<T: Resource + Clone + Debug>() -> SyncSignal<DioxusRes<T>> {
    // warn!("moving to future for resource");

    let props= use_context::<DioxusProps>();


    let mut resource_signals = use_context::<ResourceSignals>();
    // let mut dioxus_rx_registry = use_context::<DioxusRxChannels>();

    let signal = {
        let mut dioxus_resource_copies = resource_signals.0.write();

        let mut signal_registry = dioxus_resource_copies.0.write();

        let value = signal_registry.get::<T>();
        let Some(signal) = value else {
            warn!("requesting resource channel");
            return request_resource_channel(props, signal_registry);
        } ;
        signal.clone()
    };

    use_future(move || {{
        {
        let value = props.clone();
        async move {
            let mut signal = signal.clone();
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
        }
        }
    });
    signal
}

pub fn dioxus_app(props: DioxusProps) -> Element {
    // let mut state = use_context_provider(UiState::default);
    
    let register_updates = use_context_provider(||props);


    // let mut dioxus_tx_registry = use_context_provider(||DioxusTxChannels::default());
    // let mut dioxus_rx_registry = use_context_provider(||DioxusRxChannels::default());
    let mut resource_registers = use_context_provider(||ResourceSignalRegistry::default());
    let mut dioxus_panels = use_context_provider(||DioxusPanels::default());
    let mut resource_signals = use_context_provider(||ResourceSignals::default());

    let update_frequency = 1000;

    use_future(move || {
        {
        let value = register_updates.dioxus_panel_updates.clone();
        async move {
            loop {
                // Update UI every 1s in this demo.
                sleep(std::time::Duration::from_millis(update_frequency)).await;

                while let Ok(messages) = value.try_recv() {
                    let mut dioxus_panels = dioxus_panels.0.write();
                    for update in messages.0 {
                        match update.value {
                            PanelUpdateKind::Add(dioxus_panel) => {
                                dioxus_panels.insert(update.key, dioxus_panel);
                            },
                            PanelUpdateKind::Remove => {
                               dioxus_panels.remove(&update.key);
                            },
                        }
                    }
                }
            }
        }
        }
    });

    rsx! {
        for (_, panel_kind) in dioxus_panels.0.read().clone() {
            {panel_kind.element_marker.as_ref().element()}
        }
    }
}