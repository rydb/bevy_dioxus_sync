use std::collections::HashMap;

use bevy_dioxus_render::DioxusMessage;
use bevy_ecs::prelude::*;

//TODO: Move pub(crate) to pub once bevy_dioxus_panels is implemented.
pub(crate) mod net_provider;
pub mod panels;
pub mod plugins;
pub(crate) mod systems;
pub mod ui;
