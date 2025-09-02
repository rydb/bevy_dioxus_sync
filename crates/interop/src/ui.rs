use std::collections::HashMap;

use crate::hooks::asset_single::hook::BevyWrappedAssetsSignals;
use crate::hooks::component_single::hook::BevyComponentSignletonSignals;
// use crate::hooks::one_component_kind::hook::BevyComponentsSignals;
use crate::resource_sync::hook::ResourceSignals;
use crate::{DioxusPanel, plugins::DioxusProps, systems::PanelUpdateKind};
use async_std::task::sleep;
use bevy_ecs::entity::Entity;
use dioxus::prelude::*;

/// refresh rate for info sent to dioxus.
#[derive(Clone)]
pub struct InfoRefershRateMS(pub u64);

#[derive(Clone, Default)]
pub struct DioxusPanels(pub Signal<HashMap<Entity, DioxusPanel>>);

pub fn dioxus_app(props: DioxusProps) -> Element {
    let refresh_rate_ms = 1000 / props.fps.clone();

    let _info_refresh_rate = use_context_provider(|| InfoRefershRateMS(refresh_rate_ms.into()));
    let register_updates = use_context_provider(|| props);

    let _resource_registers = use_context_provider(|| ResourceSignals::default());
    // let component_signals = use_context_provider(|| BevyComponentsSignals::default());
    let mut dioxus_panels = use_context_provider(|| DioxusPanels::default());
    let _asset_singletons = use_context_provider(|| BevyWrappedAssetsSignals::default());
    let _component_singletons = use_context_provider(|| BevyComponentSignletonSignals::default());

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

    rsx! {
        for (_, panel_kind) in dioxus_panels.0.read().clone() {
            {panel_kind.element_marker.as_ref().element()}
        }
    }
}
