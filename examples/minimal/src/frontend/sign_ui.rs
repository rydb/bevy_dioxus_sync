use bevy_color::Color;
use bevy_ecs::{entity::Entity, query::With};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use dioxus::prelude::*;
use dioxus_bevy_signals::{
    query::single::use_bevy_single,
    resource::use_bevy_resource,
};

use crate::backend::{DynamicCube, SignDistance};

#[derive(Debug)]
pub struct SignUi;

const DISTANCE_INCREMENT: f32 = 1.0;
#[component]
pub fn sign_ui() -> Element {
    let cube_distance = use_bevy_resource::<SignDistance>();
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
            id: "panel",
            class: "catch-events",
            document::Stylesheet { href: asset!("src/frontend/ui.css") },
            h1 {
                "world space dom"
            }
            div {
                id: "distance-control",
                label { "Cube Distance:" }
                div {
                    class: "stepper-row",
                    button {
                        class: "stepper-btn",
                        onpointerdown: decrement,
                        "-"
                    }
                    span {
                        class: "stepper-value",
                        "{cube_distance}"
                    }
                    button {
                        class: "stepper-btn",
                        onpointerdown: increment,
                        {
                            println!("incrementing sign by one");
                            "+"
                        }
                    }
                }
            }
            h3 {
                "Alternate Cube Colors:"
            }
           div {
                id: "buttons",
                button {
                    background: "purple",
                    class: "color-button",
                    onpointerdown: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(0.502, 0.0, 0.502, 1.0)))
                    },
                }
                button {
                    background: "yellow",
                    class: "color-button",
                    onpointerdown: move |_| {
                        cube_color.mutate(|color| *color = StandardMaterial::from_color(Color::srgba(1.0, 1.0, 0.0, 1.0)))
                    },
                }
            }
        }

    }
}
