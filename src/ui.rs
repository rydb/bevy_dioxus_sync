use std::any::{type_name, TypeId};

use async_std::task::sleep;
use bevy_log::warn;
use bevy_platform::collections::HashMap;
use crossbeam_channel::{Receiver, Sender};
use dioxus::prelude::*;
use crate::{dioxus_in_bevy_plugin::DioxusProps, DioxusRxChannelsUntyped, DioxusTxChannelsUntyped, ErasedSubGenericMap};


#[derive(Default, Clone)]
pub struct UiRegisters {
    // pub bevy_io_registry: Signal<BevyIOChannels>,
    // pub dioxus_io_registry: Signal<DioxusIOChannels>
    pub dioxus_tx_registry: Signal<DioxusTxChannelsUntyped>,
    pub dioxus_rx_registry: Signal<DioxusRxChannelsUntyped>,
}

pub struct UiState {
    
}

pub fn dioxus_app(props: DioxusProps) -> Element {
    // let mut state = use_context_provider(UiState::default);
    let mut registers = use_context_provider(UiRegisters::default);

    let register_updates = use_context_provider(||props);


    use_future(move || {
        {
        let value = register_updates.dioxus_txrx.clone();
        async move {
            loop {
                // Update UI every 1s in this demo.
                sleep(std::time::Duration::from_millis(1000)).await;

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


    let p = [0, 1, 2, 3, 4, 5];
    rsx! {
        for i in p {
            {println!("i is {:#}", i);}
        }
        //app_ui {}
    }
}