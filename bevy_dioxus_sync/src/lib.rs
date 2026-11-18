use bevy_dioxus_render::DioxusWindowUiQuad;
use bevy_dioxus_render::layout::DioxusUi;
use bevy_dioxus_tracing::{error, trace};
use bevy_ecs::prelude::*;

use crate::plugins::LayoutSystem;

pub mod plugins;

#[derive(Resource)]
pub struct InitialWindowLayout(pub Option<LayoutSystem>);

/// Setup initial ui requested by plugin
fn setup_initial_window_ui(
    mut windows: Query<&mut DioxusUi, With<DioxusWindowUiQuad>>,
    initial_layout: Res<InitialWindowLayout>,
) {
    let len = windows.iter().len();
    if len != 1 {
        error!(
            "window setup requires no more and no less than one window, but got {}. TODO: Decide how to handle multiple windows",
            len
        );
    }
    let Some(mut window) = windows.iter_mut().next() else {
        trace!("no window detected, skipping initial layout setup");
        return;
    };
    window.layout = initial_layout.0.clone();
}
