//! minimal example showing each of the hooks

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_dioxus_sync::{panels::DioxusPanel, plugins::DioxusPlugin};
use tracing_subscriber::{fmt, prelude::*, registry, filter::filter_fn};

use crate::{backend::bevy_scene_plugin::BevyScenePlugin, frontend::AppUi};

pub mod backend;
pub mod frontend;

pub fn main() {
    // Only show dioxus_bevy_signals trace/debug logs, suppress everything else
    let filter = filter_fn(|metadata| {
        metadata.target().starts_with("dioxus_bevy_signals")
    });

    let stdout_layer = fmt::layer().with_writer(std::io::stdout);

    let subscriber = registry().with(filter).with(stdout_layer);

    subscriber.init();

    App::new()
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_plugins(DioxusPlugin {
            bevy_info_refresh_fps: 30,
            main_window_ui: Some(DioxusPanel::new(AppUi {})),
        })
        .add_plugins(BevyScenePlugin)
        .run();
}
