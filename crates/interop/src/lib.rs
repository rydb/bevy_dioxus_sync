use std::collections::HashMap;

use bevy_ecs::prelude::*;
// use dioxus_native_dom::DioxusDocument;
use dioxus_native::DioxusDocument;

pub struct DioxusDocuments(pub HashMap<Entity, DioxusDocument>);
