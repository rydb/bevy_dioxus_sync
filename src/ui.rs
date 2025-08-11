use std::any::{type_name, TypeId};

use async_std::task::sleep;
use bevy_ecs::entity::Entity;
use bevy_log::warn;
use bevy_platform::collections::HashMap;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus::prelude::*;
use bevy_utils::default;
use crate::{dioxus_in_bevy_plugin::{DioxusProps, SyncMessage}, systems::PanelUpdateKind, AnytypeMap, DioxusPanel, DioxusRxChannelsUntyped, DioxusTxChannelsUntyped, ErasedSubGenericMap};
use bevy_ecs::prelude::Resource;

#[derive(TransparentWrapper, Default)]
#[repr(transparent)]
pub struct SignalRegistry(AnytypeMap);

impl ErasedSubGenericMap for SignalRegistry {
    type Generic<T: Send + Sync + 'static> = SyncSignal<T>;
}

/// Dioxus signals of Dioxus copies of Bevy resources.
#[derive(Default)]
pub struct ResourceSignalRegistry(SignalRegistry);


#[derive(Clone, Default)]
pub struct UiRegisters {
    // pub bevy_io_registry: Signal<BevyIOChannels>,
    pub dioxus_panels: Signal<HashMap<Entity, DioxusPanel>>,
    pub dioxus_tx_registry: Signal<DioxusTxChannelsUntyped>,
    pub dioxus_rx_registry: Signal<DioxusRxChannelsUntyped>,
    pub resource_signal_registry: Signal<ResourceSignalRegistry>
}

pub struct DioxusRes<T: Clone + Resource> {
    resource_write: Sender<T>,
    // receiver: Receiver<T>,
    resource_read: T
}

impl<T: Clone + Resource> DioxusRes<T> {
    pub fn set(&mut self, value: T) {
        self.resource_write.send(value.clone()).unwrap();
        self.resource_read = value
    }
    // pub fn read(&mut self) -> &T {
    //     if let Ok(value) = self.receiver.try_recv() {
    //         self.resource = value.clone();
    //         &self.resource
    //     } else {
    //         &self.resource
    //     }
    // }
}

/// requests a resource from bevy. 
pub fn use_resource<T: Resource + Clone>(f: T) -> DioxusRes<T> {
    //let context = try_consume_context::<DioxusRes<T>>();
        let mut registers = use_context::<UiRegisters>();



        use_future(move || {
        async move {
            loop {
                if let Some(receiver) = registers.dioxus_rx_registry.write().0.get::<T>() {
                    if let Some(sender) = registers.dioxus_tx_registry.write().0.get::<T>() {
                        while let Ok(value) = receiver.try_recv() {
                            va
                        }
                    } else {
                        todo!("implement this properly")
                    }                 
                } else {
                    let props= use_context::<DioxusProps>();
                    props.sync_tx.send(SyncMessage::RequestResourceChannel(Box::new(f.clone())));
                    break
                }
                
                // let bevy_receiver = registers.dioxus_rx_registry.write().0.get::<FPS>().clone();
            }
        }
    });
}

// pub struct DioxRes<T> {
//     res: Write<T>
// }

pub fn dioxus_app(props: DioxusProps) -> Element {
    // let mut state = use_context_provider(UiState::default);
    
    let register_updates = use_context_provider(||props);


    let mut registers = use_context_provider(||UiRegisters {
        ..default()
    });



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
                    let mut tx_registrations = registers.dioxus_tx_registry.write();
                    let mut rx_registrations = registers.dioxus_rx_registry.write();
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
                    let mut dioxus_panels = registers.dioxus_panels.write();
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
        for (_, panel_kind) in registers.dioxus_panels.read().clone() {
            {panel_kind.element_marker.as_ref().element()}
        }
    }
}