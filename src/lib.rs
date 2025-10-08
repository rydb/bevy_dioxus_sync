use bevy_ecs::{
    component::{Component, Immutable, StorageType}, 
};
use bevy_log::warn;
use crossbeam_channel::{Receiver, Sender};
use std::{
    any::{Any, type_name},
    sync::Arc,
};

use crate::{
    systems::{DioxusPanelUpdates, PanelUpdate, PanelUpdateKind},
};

use std::fmt::Debug;

use dioxus_core::Element;

pub mod plugins;
pub(crate) mod systems;
pub mod ui;


pub struct SenderReceiver<T: Send + Sync + 'static> {
    pub sender: Sender<T>,
    pub receiver: Receiver<T>,
}

/// marks a struct as a Dioxus element.
/// used to statically typed dioxus [`Element`]s
pub trait DioxusElementMarker: 'static + Sync + Send + Debug {
    //const ELEMENT_FUNCTION: fn() -> Element;
    fn element(&self) -> Element;
}


/// Component that marks an entity as a dioxus panel
#[derive(Clone, Debug)]
pub struct DioxusPanel {
    pub(crate) element_marker: Arc<dyn DioxusElementMarker>,
}

impl DioxusPanel {
    pub fn new<T: DioxusElementMarker>(element: T) -> Self {
        Self {
            element_marker: Arc::new(element),
        }
    }
}

impl Component for DioxusPanel {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    /// to change the panel on this entity, insert a new one.
    type Mutability = Immutable;
}
pub struct ResourceUpdates {}

pub type BoxSync = Box<dyn Any + Send + Sync + 'static>;
