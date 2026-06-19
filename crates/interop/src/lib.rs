use std::collections::HashMap;

use bevy_ecs::prelude::*;
use dioxus_native_dom::DioxusDocument;


pub struct DioxusDocuments(pub HashMap<Entity, DioxusDocument>);
