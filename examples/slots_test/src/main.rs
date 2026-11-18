//! Visual test for the five slot surface.

use bevy::input::keyboard::KeyCode as BevyKey;
use bevy::prelude::*;
use bevy_dioxus_render::layout::DioxusUi;
use bevy_dioxus_render::slots::{
    Close, DARK, EdgeResizeRule, FiveSlots, FloatPanel, FreePanelPosition, LIGHT, Minimize,
    Percent, PercentSize, Slot, SlotState, Tab,
};
use bevy_dioxus_render::{active_tab, title_bar};
use bevy_dioxus_sync::plugins::{DioxusPlugin, LayoutSystem};
use dioxus::prelude::*;

#[Tab(inspector_ui)]
struct Inspector;

#[Tab(console_ui)]
struct Console;

#[Tab(debug_ui)]
struct Debug;

#[Tab(hints_ui)]
struct Hints;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.build().disable::<bevy::log::LogPlugin>())
        .add_plugins(DioxusPlugin {
            bevy_info_refresh_fps: 30,
            dioxus_render_fps_cap: 60,
            main_window_ui: Some(LayoutSystem::FiveSlots(initial_slots())),
        })
        .add_systems(Startup, setup_scene)
        .add_systems(Update, toggle_theme)
        .run();
}

fn toggle_theme(keys: Res<ButtonInput<BevyKey>>, mut ui: Query<&mut DioxusUi>) {
    if !keys.just_pressed(BevyKey::F1) {
        return;
    }
    let Ok(mut ui) = ui.single_mut() else {
        return;
    };
    let Some(LayoutSystem::FiveSlots(slots)) = ui.layout.as_mut() else {
        return;
    };
    println!("switching theme");
    slots.theme = if slots.theme.id == DARK.id {
        LIGHT
    } else {
        DARK
    };
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        AmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
            ..default()
        },
        Transform::from_xyz(3.5, 2.5, 5.0).looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Scene Light"),
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Name::new("Platform"),
        Mesh3d(meshes.add(Cuboid::new(4.0, 0.2, 4.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.24, 0.27, 0.31))),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));

    commands.spawn((
        Name::new("Cube"),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial::from_color(Srgba::GREEN))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
}

fn initial_slots() -> FiveSlots {
    let mut slots = FiveSlots::default();
    slots.left = Slot {
        title_bar: title_bar!([Inspector], [Minimize, Close]),
        active_tab: Some(active_tab!(Inspector)),
        state: SlotState::Open,
    };
    slots.right = Slot {
        title_bar: title_bar!([Debug], [Minimize, Close]),
        active_tab: Some(active_tab!(Debug)),
        state: SlotState::Open,
    };
    slots.bottom = Slot {
        title_bar: title_bar!([Console], [Minimize]),
        active_tab: Some(active_tab!(Console)),
        state: SlotState::Open,
    };
    slots.edge_sizes.bottom = Percent::new(18);
    slots.edge_resize.bottom = EdgeResizeRule::ResizeNoMinimize;

    let _ = slots.float_panels.insert(FloatPanel {
        position: FreePanelPosition::Center,
        size: PercentSize::new(30, 16),
        slot: Slot {
            title_bar: title_bar!([Hints], [Minimize, Close]),
            active_tab: Some(active_tab!(Hints)),
            ..Slot::default()
        },
        ..FloatPanel::default()
    });

    slots
}

fn inspector_ui() -> Element {
    let mut count = use_signal(|| 0);
    rsx! {
        div {
            width: "100%",
            height: "100%",
            padding: "10px",
            box_sizing: "border-box",
            font_family: "monospace",
            color: "inherit",
            h2 { "Inspector" }
            p { "Panel state survives moves while it stays mounted." }
            p { "Counter: {count}" }
            button {
                onclick: move |_| {
                    *count.write() += 1;
                },
                "Increment"
            }
        }
    }
}

fn console_ui() -> Element {
    rsx! {
        div {
            width: "100%",
            height: "100%",
            padding: "10px",
            box_sizing: "border-box",
            font_family: "monospace",
            color: "inherit",
            overflow: "auto",
            h2 { "Console" }
            p { "Inactive tab content stays mounted and hidden." }
            for line in 0..40 {
                p { "Log line {line}" }
            }
        }
    }
}

fn debug_ui() -> Element {
    let mut enabled = use_signal(|| false);
    rsx! {
        div {
            width: "100%",
            height: "100%",
            padding: "10px",
            box_sizing: "border-box",
            font_family: "monospace",
            color: "inherit",
            h2 { "Debug" }
            p { "Placeholder widget for the right column." }
            button {
                onclick: move |_| {
                    let next = !*enabled.read();
                    *enabled.write() = next;
                },
                if *enabled.read() { "Enabled" } else { "Disabled" }
            }
        }
    }
}

fn hints_ui() -> Element {
    rsx! {
        div {
            width: "100%",
            height: "100%",
            padding: "10px",
            box_sizing: "border-box",
            font_family: "monospace",
            color: "inherit",
            h2 { "Hints" }
            p { "press x to close, press - to minimize" }
        }
    }
}
