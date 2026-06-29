use std::collections::HashMap;

// use crate::plugins::DioxusPluginProps;
// use crate::panels::{DioxusPanel, PanelUpdateKind};
use async_std::task::sleep;
use bevy_dioxus_tracing::trace;
use bevy_ecs::entity::Entity;
use dioxus_core::Element;
use dioxus_core_macro::rsx;
use dioxus_hooks::{use_context_provider, use_future, use_signal};
use dioxus_signals::*;

// #[derive(Clone, Default)]
// pub struct DioxusPanels(pub Signal<HashMap<Entity, DioxusPanel>>);

// pub fn dioxus_app(props: DioxusPluginProps) -> Element {
//     let update_frequency = 1000;
//     let dioxus_props = use_context_provider(|| props.clone());
//     let mut dioxus_panels = use_context_provider(|| DioxusPanels::default());

//     let _command_queue_tx = use_context_provider(|| props.command_queue_sender);
//     use_future(move || {
//         {
//             trace!("spawning dioxus app future");
//             let value = dioxus_props.dioxus_panel_updates.clone();
//             async move {
//                 loop {
//                     sleep(std::time::Duration::from_millis(update_frequency)).await;
//                     while let Ok(messages) = value.try_recv() {
//                         trace!("panel update received: {:#?}", messages);

//                         let mut dioxus_panels = dioxus_panels.0.write();
//                         for update in messages.0 {
//                             match update.value {
//                                 PanelUpdateKind::Add(dioxus_panel) => {
//                                     dioxus_panels.insert(update.key, dioxus_panel);
//                                 }
//                                 PanelUpdateKind::Remove => {
//                                     dioxus_panels.remove(&update.key);
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     });

//     // Only insert the main window panel once to avoid infinite re-renders.
//     // Without this guard, each re-render would write to dioxus_panels again,
//     // generating a new signal revision and triggering yet another re-render.
//     let mut did_insert_main_panel = use_signal(|| false);
//     if !did_insert_main_panel() {
//         let main_window_ui = dioxus_props.main_window_ui.clone();
//         if let Some((entity, panel)) = main_window_ui {
//             dioxus_panels.0.write().insert(entity, panel);
//         }
//         did_insert_main_panel.set(true);
//     }
//     rsx! {
//         // {main_window_ui.map(|n| n())},
//         for (_, panel_kind) in dioxus_panels.0.read().clone() {
//             {panel_kind.element_marker.as_ref().element()}
//         }
//     }
// }
