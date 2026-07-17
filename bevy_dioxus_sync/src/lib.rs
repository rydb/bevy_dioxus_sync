
use bevy_dioxus_render::{DioxusWindowUiQuad, panels::DioxusPanels};
use bevy_dioxus_tracing::error;
use bevy_ecs::prelude::*;
use dioxus_core::Element;

pub mod plugins;

#[derive(Resource)]
pub struct InitialWindowPanel(pub Option<fn() -> Element>);

/// setups initial ui requested by plugin
fn setup_initial_window_ui(
    mut windows: Query<&mut DioxusPanels, With<DioxusWindowUiQuad>>,
    initial_panel: Res<InitialWindowPanel>,
) {
    let len = windows.iter().len();
    if !len == 1 {
        error!(
            "window setup requires no more and no less than one window, but got {}. TODO: Decide how to handle multple windows",
            len
        );
    }
    let mut window = windows.iter_mut().next().unwrap();
    if let Some(panel) = initial_panel.0 {
        window.panels.insert(panel);
    }
}
