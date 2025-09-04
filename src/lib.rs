use bevy_ecs::{
    component::{ComponentHooks, Immutable, StorageType},
    prelude::*,
    schedule::ScheduleLabel,
    system::ScheduleSystem,
};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
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

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, hook| {
            let Some(value) = world.entity(hook.entity).get::<Self>() else {
                warn!(
                    "could not get {:#} on {:#}",
                    type_name::<Self>(),
                    hook.entity
                );
                return;
            };
            let value = value.clone();
            let mut panel_updates = world.get_resource_mut::<DioxusPanelUpdates>().unwrap();
            // warn!(
            //     "pushing panel update for {:#} to {:#?}",
            //     hook.entity,
            //     PanelUpdateKind::Add(value.clone())
            // );
            panel_updates.0.push(PanelUpdate {
                key: hook.entity,
                value: PanelUpdateKind::Add(value),
            })
        });
        hooks.on_remove(|mut world, hook| {
            let mut panel_updates = world.get_resource_mut::<DioxusPanelUpdates>().unwrap();
            panel_updates.0.push(PanelUpdate {
                key: hook.entity,
                value: PanelUpdateKind::Remove,
            })
        });
    }
}
pub struct ResourceUpdates {}

pub type BoxSync = Box<dyn Any + Send + Sync + 'static>;
