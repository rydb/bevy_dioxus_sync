use std::collections::HashMap;

// use crate::hooks::one_component_kind::hook::BevyComponentsSignals;
use crate::{
    panels::{DioxusPanel, PanelUpdateKind},
    // plugins::DioxusAppKind,
};
use async_std::task::sleep;
use bevy_dioxus_hooks::{
    asset::BevyAssetsRegistry, component::component_single::hook::BevyComponentsRegistry,
    resource::hook::ResourceRegistry,
};
use bevy_dioxus_interop::{BevyCommandQueueTx, InfoRefershRateMS};
use bevy_ecs::entity::Entity;
use dioxus_core::Element;
use dioxus_core_macro::rsx;
use dioxus_hooks::{use_context_provider, use_future};
use dioxus_signals::*;
use crate::plugins::DioxusPluginProps;

#[derive(Clone, Default)]
pub struct DioxusPanels(pub Signal<HashMap<Entity, DioxusPanel>>);

pub enum AppKind {}

pub fn dioxus_app(props: DioxusPluginProps) -> Element {
    
    
    // let props = match app_kind {
    //     DioxusAppKind::NativeBevy(bevy_props) => {
    let update_frequency = 1000;
    // let register_updates = use_context_provider(|| bevy_props.clone());
    let dioxus_props = use_context_provider(|| props);
    let mut dioxus_panels = use_context_provider(|| DioxusPanels::default());

    let _command_queue_tx = use_context_provider(|| BevyCommandQueueTx(dioxus_props.command_queues_tx.clone()));
    use_future(move || {
        {
            let value = dioxus_props.dioxus_panel_updates.clone();
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
            // dioxus_props
    //     }
    //     DioxusAppKind::NativeOnly(props) => props,
    // };
    let refresh_rate_ms = 1000 / dioxus_props.fps.clone();

    let _info_refresh_rate = use_context_provider(|| InfoRefershRateMS(refresh_rate_ms.into()));

    let main_window_ui = dioxus_props.main_window_ui.clone();
    if let Some((entity, panel)) = main_window_ui {
        dioxus_panels.0.write().insert(entity, panel);
    }
    let _resource_registers = use_context_provider(|| ResourceRegistry::default());
    // let component_signals = use_context_provider(|| BevyComponentsSignals::default());
    // let _asset_singletons = use_context_provider(|| BevyAssetsSignals::default());
    let _asset_registers = use_context_provider(|| BevyAssetsRegistry::default());
    let _component_registers = use_context_provider(|| BevyComponentsRegistry::default());
    // let panels = 
    rsx! {
        // {main_window_ui.map(|n| n())},
        for (_, panel_kind) in dioxus_panels.0.read().clone() {
            {panel_kind.element_marker.as_ref().element()}
        }
    }
}
