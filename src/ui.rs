use std::collections::HashMap;

// use crate::hooks::one_component_kind::hook::BevyComponentsSignals;
use crate::{plugins::{DioxusAppKind}, systems::PanelUpdateKind, DioxusPanel};
use async_std::task::sleep;
use bevy_dioxus_hooks::{asset_single::hook::BevyWrappedAssetsSignals, component_single::hook::BevyComponentSignletonSignals, resource::hook::ResourceSignals};
use bevy_dioxus_interop::{BevyCommandQueueTx, InfoRefershRateMS};
use bevy_ecs::entity::Entity;
use dioxus_core::Element;
use dioxus_core_macro::rsx;
use dioxus_hooks::{use_context_provider, use_future};
use dioxus_signals::*;

#[derive(Clone, Default)]
pub struct DioxusPanels(pub Signal<HashMap<Entity, DioxusPanel>>);

pub enum AppKind {
}

pub fn dioxus_app(app_kind: DioxusAppKind) -> Element {
    
    let props = match app_kind {
        DioxusAppKind::NativeBevy(bevy_props) => {
            let dioxus_props = bevy_props.dioxus_props.clone();
            let update_frequency = 1000;
            let register_updates = use_context_provider(|| bevy_props.clone());
            let mut dioxus_panels = use_context_provider(|| DioxusPanels::default());

            let _command_queue_tx = use_context_provider(|| BevyCommandQueueTx(bevy_props.command_queues_tx.clone()));
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
                                        }
                                        PanelUpdateKind::Remove => {
                                            dioxus_panels.remove(&update.key);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });
            dioxus_props
        },
        DioxusAppKind::NativeOnly(props) => {
            props
        },
    };
    let refresh_rate_ms = 1000 / props.fps.clone();

    let _info_refresh_rate = use_context_provider(|| InfoRefershRateMS(refresh_rate_ms.into()));

    let main_window_ui = props.main_window_ui.clone();


    let _resource_registers = use_context_provider(|| ResourceSignals::default());
    // let component_signals = use_context_provider(|| BevyComponentsSignals::default());
    let _asset_singletons = use_context_provider(|| BevyWrappedAssetsSignals::default());
    let _component_singletons = use_context_provider(|| BevyComponentSignletonSignals::default());




    
    rsx! {
        {main_window_ui.map(|n| n())},
        // for (_, panel_kind) in dioxus_panels.0.read().clone() {
        //     {panel_kind.element_marker.as_ref().element()}
        // }
    }
}
