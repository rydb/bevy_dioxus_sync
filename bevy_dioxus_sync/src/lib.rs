use std::{any::TypeId, collections::{HashMap, HashSet}};

use bevy_dioxus_render::{DioxusUiQuad, panels::DioxusPanels};
use bevy_ecs::prelude::*;
use bevy_window::Window;
use dioxus_core::Element;
use std::fmt::Debug;

// pub(crate) mod net_provider;
// pub mod panels;
pub mod plugins;
pub mod ui;

#[derive(Resource)]
pub struct InitialWindowPanel(pub Option<fn() -> Element>);

/// setups initial ui requested by plugin
fn setup_initial_window_ui(
    mut windows: Single<&mut DioxusPanels>,
    initial_panel: Res<InitialWindowPanel>,
) {
    if let Some(panel) = initial_panel.0 {
        windows.panels.insert(panel);
    }
}



// /// sync dioxus ui for a window with its latest panels
// pub fn sync_dioxus_ui_with_panels(
//     mut panels: Query<&DioxusPanels, Changed<DioxusPanels>>
// ) {
//     for changed
// }