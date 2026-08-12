use bevy_color::Color;
use bevy_ecs::{entity::Entity, query::With};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use dioxus::prelude::*;
use dioxus_bevy_signals::{query::single::use_bevy_single, resource::use_bevy_resource};

use crate::backend::{DynamicCube, SignDistance};

#[derive(Debug)]
pub struct SignUi;

const DISTANCE_INCREMENT: f32 = 1.0;

#[component]
pub fn sign_ui() -> Element {
    let cube_distance = use_bevy_resource::<SignDistance, _, _>(|n| n, |err| err);
    let (_cube_entity, cube_color) =
        use_bevy_single::<(Entity, &mut MeshMaterial3d<StandardMaterial>), With<DynamicCube>>();

    let cube_color = cube_color.use_asset();

    let increment = move |_evt| {
        cube_distance.mutate(|n| n.0 += DISTANCE_INCREMENT);
    };

    let decrement = move |_evt| {
        cube_distance.mutate(|n| n.0 -= DISTANCE_INCREMENT);
    };

    rsx! {
        div {
            class: "sign-panel catch-events",
            document::Stylesheet { href: asset!("src/frontend/sign_ui.css") },
            h1 {
                class: "sign-title",
                "world space dom"
            }
            div {
                class: "sign-control",
                label { "Cube Distance:" }
                div {
                    class: "sign-stepper-row",
                    button {
                        class: "sign-stepper-btn",
                        onpointerdown: decrement,
                        "-"
                    }
                    span {
                        class: "sign-stepper-value",
                        "{cube_distance}"
                    }
                    button {
                        class: "sign-stepper-btn",
                        onpointerdown: increment,
                        "+"
                    }
                }
            }
            h3 {
                class: "sign-section-header",
                "Alternate Cube Colors:"
            }
            div {
                class: "sign-color-row",
                button {
                    class: "sign-color-btn",
                    background: "purple",
                    onpointerdown: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(0.502, 0.0, 0.502, 1.0)))
                    },
                }
                button {
                    class: "sign-color-btn",
                    background: "yellow",
                    onpointerdown: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(1.0, 1.0, 0.0, 1.0)))
                    },
                }
            }
        }
    }
}
