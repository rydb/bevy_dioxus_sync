use std::{any::{type_name, TypeId}, sync::{mpsc::Receiver, Arc}};

use bevy_ecs::{component::{ComponentHooks, Immutable, Mutable, StorageType}, prelude::*};
use bevy_log::warn;
use bevy_platform::collections::{HashMap, HashSet};
use crossbeam_channel::Sender;
use dioxus::core::Element;
use std::fmt::Debug;

use crate::traits::DioxusElementMarker;

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

#[derive(Resource, Debug)]
pub struct DioxusPanelUpdates(pub(crate) Vec<PanelUpdate>);

pub struct ActivePanels(HashMap<Entity, DioxusPanel>);

/// Component that marks an entity as a dioxus panel
#[derive(Clone, Debug)]
pub struct DioxusPanel {
    element_marker: Arc<dyn DioxusElementMarker>
}

impl Component for DioxusPanel {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    /// to change the panel on this entity, insert a new one.
    type Mutability = Immutable;

     fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, hook| {
            let Some(value) = world.entity(hook.entity).get::<Self>() else {
                warn!("could not get {:#} on {:#}", type_name::<Self>(), hook.entity);
                return
            };
            let value = value.clone();
            let mut panel_updates = world.get_resource_mut::<DioxusPanelUpdates>().unwrap();

            panel_updates.0.push(PanelUpdate { key: hook.entity, value: PanelUpdateKind::Add(value) })
        });
        hooks.on_remove(|mut world, hook| {
            let mut panel_updates = world.get_resource_mut::<DioxusPanelUpdates>().unwrap();
            panel_updates.0.push(PanelUpdate { key: hook.entity, value: PanelUpdateKind::Remove })
        });
    }
}

impl DioxusPanel {
    pub fn new<T: DioxusElementMarker>(element: impl DioxusElementMarker) -> Self {
        Self {
            element_marker: Arc::new(element)
        }
    }
}

// /// Send Dioxus panels to be rendered
// pub fn update_dioxus_side_panels(
//     current_panels: Query<&DioxusPa
//     panels: Query<&DioxusPanel>,

// ) {
    
//     //  for panel in panels {
//     //     panels_tx.0.send(panel.clone());
//     //  }
// }


