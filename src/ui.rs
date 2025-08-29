use std::fmt::Display;

use crate::queries_sync::asset_single::hook::BevyWrappedAssetsSignals;
use crate::queries_sync::component_single::hook::BevyComponentSignletonSignals;
use crate::queries_sync::one_component_kind::hook::BevyComponentsSignals;
use crate::resource_sync::hook::ResourceSignals;
use crate::traits::ErasedSubGenericComponentsMap;
use crate::{
    BoxAnyTypeMap, DioxusPanel, ErasedSubGenericMap, dioxus_in_bevy_plugin::DioxusProps,
    systems::PanelUpdateKind,
    traits::ErasedSubGenericResourcecMap,
};
use async_std::task::sleep;
use bevy_ecs::component::Component;
use bevy_ecs::prelude::Resource;
use bevy_ecs::query::QueryData;
use bevy_ecs::{entity::Entity, world::CommandQueue};
use bevy_log::warn;
use bevy_platform::collections::HashMap;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus::prelude::*;
use std::fmt::Debug;
use bevy_ecs::prelude::*;




#[derive(Clone, Default)]
pub struct DioxusPanels(pub Signal<HashMap<Entity, DioxusPanel>>);

pub fn dioxus_app(props: DioxusProps) -> Element {

    let register_updates = use_context_provider(|| props);

    let resource_registers = use_context_provider(|| ResourceSignals::default());
    let component_signals = use_context_provider(||BevyComponentsSignals::default());
    let mut dioxus_panels = use_context_provider(|| DioxusPanels::default());
    let mut assets_wrapped = use_context_provider(||BevyWrappedAssetsSignals::default());

    let mut component_singletons = use_context_provider(|| BevyComponentSignletonSignals::default());

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
