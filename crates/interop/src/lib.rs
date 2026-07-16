use std::collections::HashMap;

use bevy_ecs::prelude::*;
use crossbeam_channel::Receiver;
use dioxus_devtools::DevserverMsg;
// use dioxus_native_dom::DioxusDocument;
use dioxus_native::DioxusDocument;

pub struct DioxusDocuments(pub HashMap<Entity, DioxusDocumentInfo>);

#[derive(Debug)]
pub struct HeadElement {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub contents: Option<String>,
}

/// Messages sent from dioxus
#[derive(Debug)]
pub enum DioxusMessage {
    Devserver(DevserverMsg),
    CreateHeadElement(HeadElement),
    ResourceLoad(blitz_dom::net::Resource),
}

/// Dioxus document + extra things required for bevy <-> dioxus interop
pub struct DioxusDocumentInfo {
    pub document: DioxusDocument,
    pub messages_recv: Receiver<DioxusMessage>,
}
