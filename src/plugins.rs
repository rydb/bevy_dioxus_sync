use std::any::type_name;

use bevy_app::prelude::*;
use bevy_dioxus_events::plugins::DioxusEventSyncPlugin;
use bevy_dioxus_interop::plugins::DioxusBevyInteropPlugin;
use bevy_dioxus_render::plugins::DioxusRenderPlugin;
use bevy_ecs::{prelude::*, world::CommandQueue};

use bevy_log::warn;
use bevy_utils::default;
use blitz_dom::DocumentConfig;
use crossbeam_channel::{Receiver, Sender};
use dioxus_core::{Element, VirtualDom};
use dioxus_native_dom::DioxusDocument;
use crate::systems::PanelUpdateKind;
use crate::PanelUpdate;
use crate::{
    systems::DioxusPanelUpdates, ui::dioxus_app, DioxusPanel,
};

pub struct DioxusPlugin {
    /// how many times per second does dioxus refresh info from bevy.
    pub bevy_info_refresh_fps: u16,

    pub main_window_ui: Option<fn() -> Element>
}
#[derive(Clone)]
pub struct DioxusPropsNative {
    pub fps: u16,
    pub main_window_ui: Option<fn() -> Element>
}

#[derive(Clone)]
pub struct DioxusPropsNativeBevy {
    pub(crate) dioxus_props: DioxusPropsNative,
    pub(crate) dioxus_panel_updates: Receiver<DioxusPanelUpdates>,
    pub(crate) command_queues_tx: Sender<CommandQueue>,
}

#[derive(Clone)]
pub enum DioxusAppKind {
    NativeBevy(DioxusPropsNativeBevy),
    NativeOnly(DioxusPropsNative)
}

/// plugin for managing dioxus plugins
pub struct DioxusPanelsPlugin {
    dioxus_panel_updates_tx: Sender<DioxusPanelUpdates>, 
    dioxus_panel_updates_rx: Receiver<DioxusPanelUpdates>
}

impl DioxusPanelsPlugin {
    pub fn new() -> Self {
        let (dioxus_panel_updates_tx, dioxus_panel_updates_rx) =
        crossbeam_channel::unbounded::<DioxusPanelUpdates>();

        Self { dioxus_panel_updates_tx, dioxus_panel_updates_rx}
    }
}

impl Plugin for DioxusPanelsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DioxusPanelUpdates>();
        app.insert_resource(DioxusPanelUpdatesSender(self.dioxus_panel_updates_tx.clone()));

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

#[derive(Resource)]
pub struct DioxusPanelUpdatesSender(Sender<DioxusPanelUpdates>);

impl Plugin for DioxusPlugin {
    fn build(&self, app: &mut App) {
        
        let bevy_dioxus_interop_plugin = DioxusBevyInteropPlugin::new();
        let dioxus_panels_plugin = DioxusPanelsPlugin::new();
        let props = DioxusPropsNativeBevy {
            dioxus_panel_updates: dioxus_panels_plugin.dioxus_panel_updates_rx.clone(),
            command_queues_tx: bevy_dioxus_interop_plugin.command_tx.clone(),
            dioxus_props: DioxusPropsNative {
                fps: self.bevy_info_refresh_fps,
                main_window_ui: self.main_window_ui
            }

        };
        app.add_plugins(bevy_dioxus_interop_plugin);
        app.add_plugins(DioxusRenderPlugin);
        app.add_plugins(DioxusEventSyncPlugin);
        app.add_plugins(dioxus_panels_plugin);

        // Create the dioxus virtual dom and the dioxus-native document
        // The viewport will be set in setup_ui after we get the window size
        let vdom = VirtualDom::new_with_props(dioxus_app, DioxusAppKind::NativeBevy(props));
        // FIXME add a NetProvider
        let mut dioxus_doc = DioxusDocument::new(vdom, DocumentConfig {
            ..default()
        });
        dioxus_doc.initial_build();
        dioxus_doc.resolve(0.0);
        app.insert_non_send_resource(dioxus_doc);


    }
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
