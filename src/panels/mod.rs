use std::sync::Arc;

use bevy_ecs::{
    component::{Immutable, StorageType},
    prelude::*,
};
use bevy_log::warn;
use crossbeam_channel::Sender;
use dioxus_core::Element;
use std::fmt::Debug;

pub mod plugins;

#[derive(Resource)]
pub struct DioxusPanelUpdatesSender(Sender<DioxusPanelUpdates>);

/// Component that marks an entity as a dioxus panel
#[derive(Clone, Debug)]
pub struct DioxusPanel {
    pub(crate) _element_marker: Arc<dyn DioxusElementMarker>,
}

/// marks a struct as a Dioxus element.
/// used to statically typed dioxus [`Element`]s
pub trait DioxusElementMarker: 'static + Sync + Send + Debug {
    //const ELEMENT_FUNCTION: fn() -> Element;
    fn element(&self) -> Element;
}

#[derive(Debug)]
pub struct PanelUpdate {
    pub(crate) key: Entity,
    pub(crate) value: PanelUpdateKind,
}

#[derive(Debug)]
pub enum PanelUpdateKind {
    Add(DioxusPanel),
    Remove,
}

#[derive(Resource, Debug, Default)]
pub struct DioxusPanelUpdates(pub(crate) Vec<PanelUpdate>);

impl DioxusPanel {
    pub fn new<T: DioxusElementMarker>(element: T) -> Self {
        Self {
            _element_marker: Arc::new(element),
        }
    }
}

impl Component for DioxusPanel {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    /// to change the panel on this entity, insert a new one.
    type Mutability = Immutable;
}

pub fn push_panel_updates(
    mut panel_updates: ResMut<DioxusPanelUpdates>,
    panel_update_sender: ResMut<DioxusPanelUpdatesSender>,
) {
    let mut updates = Vec::new();

    updates.extend(panel_updates.0.drain(..));

    let _ = panel_update_sender
        .0
        .send(DioxusPanelUpdates(updates))
        .inspect_err(|err| warn!("{:#}", err));
}
