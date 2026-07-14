use crate::backend::*;
use bevy_color::{Color, Srgba};
// use bevy_dioxus_sync::panels::DioxusElementMarker;
use bevy_ecs::{entity::Entity, query::With};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_transform::components::Transform;
use dioxus::prelude::*;
use dioxus_bevy_signals::macros::debug;
use dioxus_bevy_signals::{
    asset::{AssetNoneState, use_bevy_asset},
    query::single::use_bevy_single,
    resource::use_bevy_resource,
};

#[derive(Debug)]
pub struct AppUi;

pub const QUAT_CHAR_INDEX: [&'static str; 4] = ["x", "y", "z", "w"];

#[component]
pub fn app_ui() -> Element {
    let fps = use_bevy_resource::<FPS>();

    let cube = use_bevy_single::<
        (
            Entity,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<DynamicCube>,
    >();

    #[allow(unused)]
    let _cube_dbg2 = use_memo(move || {
        let r = cube.read_ok(|n| Ok(n.2.read().0.id())).unwrap_or_else(|err| Err(format!("{:?}", err)));
        debug!("ui: color_handle={:?}", r);
    });

    let cube_translation_str = use_memo(move || {
        cube.read_ok(|n| {
            let t = &n.1.read().translation;
            format!("{:>5.2} {:>5.2} {:>5.2}", t.x, t.y, t.z)
        })
        .unwrap_or_else(|err| err.into())
    });

    let cube_color_handle = use_memo(move || {
        cube.read_ok(|n| Ok(n.2.read().0.id()))
            .unwrap_or_else(|err| Err(AssetNoneState::Error(err.into())))
    });
    let cube_color = use_bevy_asset(cube_color_handle);

    let cube_rotation_speed = use_bevy_resource::<CubeRotationSpeed>();
    let cube_translation_speed = use_bevy_resource::<CubeTranslationSpeed>();

    // prevents cursor jumping due to blitz dom rendering issues
    let mut translation_edited = use_signal(|| false);
    let mut rotation_edited = use_signal(|| false);
    let mut edit_translation = use_signal(String::new);
    let mut edit_rotation = use_signal(String::new);

    // Memos recompute synchronously on every render, avoiding the race
    // between use_effect and the initial bevy resource sync.
    let translation_speed_str = use_memo(move || {
        if *translation_edited.read() {
            edit_translation.read().clone()
        } else {
            cube_translation_speed
                .read_ok(|n| n.0.to_string())
                .unwrap_or_else(|_| "???".to_string())
        }
    });
    let rotation_speed_str = use_memo(move || {
        if *rotation_edited.read() {
            edit_rotation.read().clone()
        } else {
            cube_rotation_speed
                .read_ok(|n| n.0.to_string())
                .unwrap_or_else(|_| "???".to_string())
        }
    });

    let set_rotation_speed = move |evt: Event<FormData>| {
        let val = evt.value();
        rotation_edited.set(true);
        edit_rotation.set(val.clone());
        if let Ok(speed) = val.parse::<f32>() {
            cube_rotation_speed.mutate(move |n| *n = CubeRotationSpeed(speed));
        }
    };

    let set_translation_speed = move |evt: Event<FormData>| {
        let val = evt.value();
        translation_edited.set(true);
        edit_translation.set(val.clone());
        if let Ok(speed) = val.parse::<f32>() {
            cube_translation_speed.mutate(move |n| *n = CubeTranslationSpeed(speed));
        }
    };

    let rgba_css = use_memo(move || {
        let rgba = match &**cube_color.read() {
            Ok(value) => &value.base_color.to_srgba(),
            Err(_) => &Srgba::default(),
        };
        format!(
            "rgba({}, {}, {}, {})",
            (rgba.red * 255.0) as u8,
            (rgba.green * 255.0) as u8,
            (rgba.blue * 255.0) as u8,
            rgba.alpha,
        )
    });

    let value = rsx! {
        document::Stylesheet { href: asset!("src/frontend/ui.css") }
        div {
            id: "panel",
            class: "catch-events",
            div {
                id: "title",
                h1 {
                   u {
                    "bevy_dioxus_sync: "
                   }
                   br {}
                   b {"example menu "}
                }
            }
            div {
                id: "buttons",
                button {
                    background: "red",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(1.0, 0.0, 0.0, 1.0)));
                    },
                }
                button {
                    background: "green",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(0.0, 1.0, 0.0, 1.0)))
                    },
                }
                button {
                    background: "blue",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(0.0, 0.0, 1.0, 1.0)))
                    },
                }
            }
            div {
                class: "section-header",
                "Status"
            }
            div {
                id: "rotation-display",
                label {
                    {"Cube Position: ".to_string()}
                }
                label {
                    class: "bevy-display",
                    {
                        {cube_translation_str}
                    }
                }
            }
            div {
                class: "section-header",
                "Controls"
            }
            div {
                id: "translation-speed-control",
                label { "Translation Speed:" }
                input {
                    r#type: "number",
                    min: "0.0",
                    max: "10.0",
                    step: "0.1",
                    value: translation_speed_str,
                    oninput: set_translation_speed,
                }
            }
            div {
                id: "rotation-speed-control",
                label { "Rotation Speed:" }
                input {
                    r#type: "number",
                    min: "0.0",
                    max: "10.0",
                    step: "0.1",
                    value: rotation_speed_str,
                    oninput: set_rotation_speed,
                }
            }

            div {
                flex: "0 0 150px",
                display: "grid",
                align_items: "center",
                justify_items: "center",
                div {
                    class: "spin-box",
                    background: "{rgba_css}",
                }
            }
            div {
                id: "footer",
                p { "Bevy framerate: {fps}" }
            }
        }
    };
    value
}
