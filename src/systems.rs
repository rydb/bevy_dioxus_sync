use bevy_ecs::prelude::*;
use std::fmt::Debug;

use crate::DioxusPanel;

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