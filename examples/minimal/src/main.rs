//! minimal example showing each of the hooks

use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_dioxus_sync::plugins::DioxusPlugin;
use bevy_picking::{PickingPlugin, input::PointerInputPlugin, mesh_picking::MeshPickingPlugin};
use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::{filter::filter_fn, fmt::{self, format::FmtSpan}, prelude::*, registry};

use crate::{backend::bevy_scene_plugin::BevyScenePlugin, frontend::ui::app_ui};

pub mod backend;
pub mod frontend;

pub fn main() {
    // general testing filter:
    let stdout_filter = filter_fn(|metadata| {
        metadata.target().starts_with("bevy_dioxus_render")
            || metadata.target().starts_with("dioxus_bevy_signals")
            || metadata.target().starts_with("minimal")
    });

    // performance logs filter:
    // let chrome_filter = filter_fn(|_metadata| true);
    // let _ = std::fs::remove_file("./target/chrome_trace.json").inspect_err(|err| warn!("{err}"));

    // let (chrome_layer, _chrome_guard) = ChromeLayerBuilder::new()
    //     .file("./target/chrome_trace.json")
    //     .include_args(true)
    //     .build();
    // let chrome_layer = chrome_layer.with_filter(chrome_filter);

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(stdout_filter);

    let subscriber = registry()
        // .with(chrome_layer)
        .with(stdout_layer);

    subscriber.init();

    App::new()
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_plugins((
            PointerInputPlugin,
            PickingPlugin,
            MeshPickingPlugin,
        ))
        .add_plugins(DioxusPlugin {
            bevy_info_refresh_fps: 30,
            main_window_ui: Some(app_ui),
        })
        .add_plugins(BevyScenePlugin)
        .run();
}
