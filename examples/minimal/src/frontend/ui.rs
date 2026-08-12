use crate::backend::*;
use bevy_color::{Color, Srgba};
use bevy_ecs::{entity::Entity, query::With};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_transform::components::Transform;
use dioxus::prelude::*;
use dioxus_bevy_signals::{query::single::use_bevy_single, resource::use_bevy_resource};

#[derive(Debug)]
pub struct AppUi;

pub const QUAT_CHAR_INDEX: [&'static str; 4] = ["x", "y", "z", "w"];

#[component]
pub fn app_ui() -> Element {
    let fps = use_bevy_resource::<FPS, _, _>(|n| n, |err| err);

    let (_cube_entity, cube_transform, cube_color) = use_bevy_single::<
        (
            Entity,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<DynamicCube>,
    >();

    let cube_translation_str = cube_transform.use_display(|t| {
        format!(
            "{:>5.2} {:>5.2} {:>5.2}",
            t.translation.x, t.translation.y, t.translation.z
        )
    });
    let cube_color = cube_color.use_asset();

    let cube_rotation_speed = use_bevy_resource::<CubeRotationSpeed, _, _>(|n| n, |err| err);
    let cube_translation_speed = use_bevy_resource::<CubeTranslationSpeed, _, _>(|n| n, |err| err);

    let set_rotation_speed = move |evt: Event<FormData>| {
        if let Ok(speed) = evt.value().parse::<f32>() {
            cube_rotation_speed.mutate(move |n| *n = CubeRotationSpeed(speed));
        }
    };

    let set_translation_speed = move |evt: Event<FormData>| {
        if let Ok(speed) = evt.value().parse::<f32>() {
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
                    onpointerdown: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(1.0, 0.0, 0.0, 1.0)));
                    },
                }
                button {
                    background: "green",
                    class: "color-button",
                    onpointerdown: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(0.0, 1.0, 0.0, 1.0)))
                    },
                }
                button {
                    background: "blue",
                    class: "color-button",
                    onpointerdown: move |_| {
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
                    value: cube_translation_speed,
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
                    value: cube_rotation_speed,
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
