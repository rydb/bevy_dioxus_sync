use std::{any::{type_name, TypeId}, fmt::Display};

use async_std::task::sleep;
use bevy_ecs::{entity::Entity, world::{CommandQueue, World}};
use bevy_log::warn;
use bevy_platform::collections::HashMap;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus::prelude::*;
use bevy_utils::default;
use crate::{dioxus_in_bevy_plugin::{DioxusCommandQueueRx, DioxusProps}, systems::PanelUpdateKind, traits::ErasedSubGeneriResourcecMap, ArcAnytypeMap, BoxAnyTypeMap, DioxusPanel, DioxusRxChannelsUntyped, DioxusTxChannelsUntyped, ErasedSubGenericMap, InsertDefaultResource, RegisterDioxusInterop};
use bevy_ecs::prelude::Resource;

#[derive(TransparentWrapper, Default, Clone)]
#[repr(transparent)]
pub struct SignalRegistry(ArcAnytypeMap);

impl ErasedSubGeneriResourcecMap for SignalRegistry {
    type Generic<T: Clone + Resource + Send + Sync + 'static> = SyncSignal<DioxusRes<T>>;
}


/// Dioxus signals of Dioxus copies of Bevy resources.
#[derive(Default, Clone)]
pub struct ResourceSignalRegistry(SignalRegistry);

#[derive(Clone, Default)]
pub struct DioxusPanels(pub Signal<HashMap<Entity, DioxusPanel>>);

#[derive(Clone, Default)]
pub struct DioxusRxChannels(pub Signal<DioxusRxChannelsUntyped>);

#[derive(Clone, Default)]
pub struct DioxusTxChannels(pub Signal<DioxusTxChannelsUntyped>);

#[derive(Clone, Default)]
pub struct ResourceSignals(pub Signal<ResourceSignalRegistry>);

pub struct DioxusRes<T: Resource> {
    pub(crate) resource_write: Sender<T>,
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
        self.resource_write.send(value.clone()).unwrap();
        self.resource_read = Some(value)
    }
    pub fn read_resource(&self) -> &Option<T> {
       &self.resource_read
    }
}

fn request_resource_channel<T:Resource>() -> SyncSignal<DioxusRes<T>> {
    let mut resource_signals = use_context::<ResourceSignals>();

    let props= use_context::<DioxusProps>();

    let command = RegisterDioxusInterop::<T>::new(crate::InsertDefaultResource::No);
    let new_signal = SyncSignal::new_maybe_sync(DioxusRes {
        resource_read: None,
        resource_write: command.dioxus_tx
    });
    let commands = CommandQueue::default();

    let mut dioxus_resource_copies = resource_signals.0.write();
    dioxus_resource_copies.0.insert(new_signal.clone());

    props.command_queues_tx.send(commands);

    return new_signal
}

/// requests a resource from bevy. 
pub fn use_bevy_resource<T: Resource + Clone>() -> SyncSignal<DioxusRes<T>> {
    warn!("moving to future for resource");

    let mut resource_signals = use_context::<ResourceSignals>();
    let mut dioxus_tx_registry = use_context::<DioxusTxChannels>();
    let mut dioxus_rx_registry = use_context::<DioxusRxChannels>();

    let mut dioxus_resource_copies = resource_signals.0.write();

    let Some(signal) = dioxus_resource_copies.0.get::<T>() else {
            return request_resource_channel()
    };
    let signal = signal.clone();

    use_future(move || {
        {
        async move {
            let mut signal = signal.clone();
            warn!("checkng for resource");
            loop {
                if let Some(receiver) = dioxus_rx_registry.0.write().0.get::<T>() {
                    if let Some(sender) = dioxus_tx_registry.0.write().0.get::<T>() {
                        let mut new_value = None;
                        while let Ok(value) = receiver.try_recv() {
                            new_value = Some(value)
                        }
                        let Some(new_value) = new_value else{
                            return signal
                        };
                        let write = signal.write().set_resource(new_value);
                        return signal;

                    } else {                        
                        return request_resource_channel()
                    }                 
                } else {
                    return request_resource_channel()
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


    let dioxus_tx_registry = use_context_provider(||DioxusTxChannels::default());
    let dioxus_rx_registry = use_context_provider(||DioxusRxChannels::default());
    let resource_registers = use_context_provider(||ResourceSignalRegistry::default());
    let dioxus_panels = use_context_provider(||DioxusPanels::default());


    let update_frequency = 1000;
    use_future(move || {
        {
        let value = register_updates.dioxus_txrx.clone();
        async move {
            loop {
                // Update UI every 1s in this demo.
                sleep(std::time::Duration::from_millis(update_frequency)).await;

                while let Ok(message) = value.try_recv() {
                    warn!("updating registry to {:#?}", message);
                    let mut tx_registrations = dioxus_tx_registry.0.write();
                    let mut rx_registrations = dioxus_rx_registry.0.write();
                    tx_registrations.0.extend(message.tx.0);
                    rx_registrations.0.extend(message.rx.0);
                }
            }
        }
        }
    });

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