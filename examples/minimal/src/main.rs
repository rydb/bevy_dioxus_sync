//! minimal example showing each of the hooks

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_dioxus_sync::{plugins::DioxusPlugin};
use tracing_subscriber::{filter::filter_fn, fmt, prelude::*, registry};

use crate::{backend::bevy_scene_plugin::BevyScenePlugin, frontend::ui::app_ui};

pub mod backend;
pub mod frontend;

pub fn main() {
    // Only show dioxus_bevy_signals trace/debug logs, suppress everything else
    // let filter = filter_fn(|metadata| metadata.target().starts_with("dioxus_bevy_signals"));

    let filter = filter_fn(|metadata| metadata.target().starts_with("bevy_dioxus_render"));

    let stdout_layer = fmt::layer().with_writer(std::io::stdout);

    let subscriber = registry()
    .with(filter)
    .with(stdout_layer);

    subscriber.init();

    App::new()
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_plugins(DioxusPlugin {
            bevy_info_refresh_fps: 30,
            main_window_ui: Some(app_ui),
        })
        .add_plugins(BevyScenePlugin)
        .run();
}
