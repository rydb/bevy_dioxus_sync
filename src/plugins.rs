use std::rc::Rc;

use bevy_app::prelude::*;
use bevy_dioxus_events::plugins::DioxusEventSyncPlugin;
use bevy_dioxus_interop::plugins::DioxusBevyInteropPlugin;
use bevy_dioxus_render::DioxusMessages;
use bevy_dioxus_render::plugins::DioxusRenderPlugin;
use bevy_ecs::world::CommandQueue;

use crate::panels::DioxusPanelUpdates;
use crate::panels::plugins::DioxusPanelsPlugin;
use crate::{ui::dioxus_app, *};
use bevy_utils::default;
use blitz_dom::DocumentConfig;
use crossbeam_channel::{Receiver, Sender};
use dioxus_core::{Element, ScopeId, VirtualDom, provide_context};
use dioxus_native_dom::DioxusDocument;

pub struct DioxusPlugin {
    /// how many times per second does dioxus refresh info from bevy.
    pub bevy_info_refresh_fps: u16,

    pub main_window_ui: Option<fn() -> Element>,
}
#[derive(Clone)]
pub struct DioxusPropsNative {
    pub fps: u16,
    pub main_window_ui: Option<fn() -> Element>,
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
    NativeOnly(DioxusPropsNative),
}

impl Plugin for DioxusPlugin {
    fn build(&self, app: &mut App) {
        let (s, r) = crossbeam_channel::unbounded();

        let bevy_dioxus_interop_plugin = DioxusBevyInteropPlugin::new();
        let dioxus_panels_plugin = DioxusPanelsPlugin::new();
        let props = DioxusPropsNativeBevy {
            dioxus_panel_updates: dioxus_panels_plugin.dioxus_panel_updates_rx.clone(),
            command_queues_tx: bevy_dioxus_interop_plugin.command_tx.clone(),
            dioxus_props: DioxusPropsNative {
                fps: self.bevy_info_refresh_fps,
                main_window_ui: self.main_window_ui,
            },
        };
        app.add_plugins(bevy_dioxus_interop_plugin);
        app.add_plugins(DioxusRenderPlugin);
        app.add_plugins(DioxusEventSyncPlugin);
        app.add_plugins(dioxus_panels_plugin);

        // Create the dioxus virtual dom and the dioxus-native document
        // The viewport will be set in setup_ui after we get the window size
        let vdom = VirtualDom::new_with_props(dioxus_app, DioxusAppKind::NativeBevy(props));

        let mut dioxus_doc = DioxusDocument::new(vdom, DocumentConfig { ..default() });
        // Setup NetProvider
        let net_provider = BevyNetProvider::shared(s.clone());

        dioxus_doc.set_net_provider(net_provider);

        // Setup DocumentProxy to process CreateHeadElement messages
        let proxy = Rc::new(DioxusDocumentProxy::new(s.clone()));
        dioxus_doc.vdom.in_scope(ScopeId::ROOT, move || {
            provide_context(proxy as Rc<dyn dioxus_document::Document>);
        });

        // Setup devtools listener for hot-reloading
        dioxus_devtools::connect(move |msg| s.send(DioxusMessage::Devserver(msg)).unwrap());
        app.insert_resource(DioxusMessages(r));

        dioxus_doc.initial_build();
        dioxus_doc.resolve(0.0);
        app.insert_non_send_resource(dioxus_doc);
    }
}
