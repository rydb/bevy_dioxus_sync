use std::{any::{type_name, TypeId}, sync::{mpsc::Receiver, Arc}};

use bevy_ecs::{component::{ComponentHooks, Immutable, Mutable, StorageType}, prelude::*};
use bevy_log::warn;
use bevy_platform::collections::{HashMap, HashSet};
use crossbeam_channel::Sender;
use dioxus::core::Element;
use std::fmt::Debug;

use crate::{traits::DioxusElementMarker, DioxusPanel};

// pub trait DioxusPanelModifier;

// pub enum PanelUpdate {
//     Remove
    
// }

#[derive(Debug)]
pub struct PanelUpdate {
    pub(crate) key: Entity,
    pub(crate) value: PanelUpdateKind
}

#[derive(Debug)]
pub enum PanelUpdateKind {
    Add(DioxusPanel),
    Remove,
}

#[derive(Resource, Debug, Default)]
pub struct DioxusPanelUpdates(pub(crate) Vec<PanelUpdate>);