use std::any::type_name;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use crossbeam_channel::{Receiver, Sender};

use crate::panels::*;

/// plugin for managing dioxus plugins
pub struct DioxusPanelsPlugin {
    pub dioxus_panel_updates_tx: Sender<DioxusPanelUpdates>,
    pub dioxus_panel_updates_rx: Receiver<DioxusPanelUpdates>,
}

impl DioxusPanelsPlugin {
    pub fn new() -> Self {
        let (dioxus_panel_updates_tx, dioxus_panel_updates_rx) =
            crossbeam_channel::unbounded::<DioxusPanelUpdates>();

        Self {
            dioxus_panel_updates_tx,
            dioxus_panel_updates_rx,
        }
    }
}

impl Plugin for DioxusPanelsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DioxusPanelUpdates>();
        app.insert_resource(DioxusPanelUpdatesSender(
            self.dioxus_panel_updates_tx.clone(),
        ));

        app.world_mut()
            .register_component_hooks::<DioxusPanel>()
            .on_add(|mut world, hook| {
                let Some(value) = world.entity(hook.entity).get::<DioxusPanel>() else {
                    warn!(
                        "could not get {:#} on {:#}",
                        type_name::<DioxusPanel>(),
                        hook.entity
                    );
                    return;
                };
                let value = value.clone();
                let mut panel_updates = world.get_resource_mut::<DioxusPanelUpdates>().unwrap();
                panel_updates.0.push(PanelUpdate {
                    key: hook.entity,
                    value: PanelUpdateKind::Add(value),
                })
            })
            .on_remove(|mut world, hook| {
                let mut panel_updates = world.get_resource_mut::<DioxusPanelUpdates>().unwrap();
                panel_updates.0.push(PanelUpdate {
                    key: hook.entity,
                    value: PanelUpdateKind::Remove,
                })
            });
        app.add_systems(
            PreUpdate,
            push_panel_updates.run_if(resource_changed::<DioxusPanelUpdates>),
        );
    }
}
