use std::rc::Rc;
use std::sync::Arc;

use bevy_app::prelude::*;
// use bevy_dioxus_hooks::resource::BevyResourcesSignalsPlugin;
use bevy_dioxus_interop::DioxusDocuments;
// use bevy_dioxus_interop::plugins::DioxusBevyInteropPlugin;
use bevy_dioxus_messages::plugins::DioxusEventSyncPlugin;
use bevy_dioxus_render::DioxusMessages;
use bevy_dioxus_render::plugins::DioxusRenderPlugin;
use bevy_ecs::world::CommandQueue;
use dioxus_bevy_signals::{CommandQueueSender, DioxusBevyMirrorPlugin};

use crate::net_provider::{BevyNetProvider, DioxusDocumentProxy};
use crate::panels::plugins::DioxusPanelsPlugin;
use crate::panels::{DioxusPanel, DioxusPanelUpdates};
use crate::{ui::dioxus_app, *};
use bevy_utils::default;
use blitz_dom::DocumentConfig;
use crossbeam_channel::{Receiver, Sender};
use dioxus_core::{ScopeId, VirtualDom, provide_context};
use dioxus_native_dom::DioxusDocument;
use linebender_resource_handle::Blob;
use parley::FontContext;

// /// Embedded DejaVu Sans font provides basic sans-serif text rendering
// /// when no system fonts are available (e.g. NixOS, containers, headless Linux).
// const DEJAVU_SANS: &[u8] = include_bytes!("../../assets/DejaVuSans.ttf");

pub struct DioxusPlugin {
    /// how many times per second does dioxus refresh info from bevy.
    pub bevy_info_refresh_fps: u32,

    pub main_window_ui: Option<DioxusPanel>,
}

#[derive(Clone)]
pub struct DioxusPluginProps {
    // pub fps: u16,
    pub main_window_ui: Option<(Entity, DioxusPanel)>,
    pub(crate) dioxus_panel_updates: Receiver<DioxusPanelUpdates>,
    pub command_queue_sender: CommandQueueSender,
}

impl Plugin for DioxusPlugin {
    fn build(&self, app: &mut App) {
        // app.add_plugins(BevyResourcesSignalsPlugin);
        let (s, r) = crossbeam_channel::unbounded();

        let mut documents = HashMap::new();


        let dioxus_signals_mirror_plugin = DioxusBevyMirrorPlugin {
            dioxus_sync_fps: self.bevy_info_refresh_fps,
            bevy_command_txrx: Default::default(),
        };
        // let bevy_dioxus_interop_plugin = DioxusBevyInteropPlugin::new();
        let dioxus_panels_plugin = DioxusPanelsPlugin::new();

        let panels_rx = dioxus_panels_plugin.dioxus_panel_updates_rx.clone();
        // panels plugin must be added before `DioxusPanels is first used or bevy will crash when adding component hooks for it`
        app.add_plugins(dioxus_panels_plugin);

        if let Some(main_window_ui) = &self.main_window_ui {
            let entity = app.world_mut().spawn(main_window_ui.clone()).id();

            let props = DioxusPluginProps {
                // fps: self.bevy_info_refresh_fps,
                main_window_ui: Some((entity, main_window_ui.clone())),
                dioxus_panel_updates: panels_rx,
                command_queue_sender: CommandQueueSender { tx: dioxus_signals_mirror_plugin.bevy_command_txrx.tx() },
                // command_queues_tx: bevy_dioxus_interop_plugin.command_tx.clone(),
            };

            // Create the dioxus virtual dom and the dioxus-native document
            // The viewport will be set in setup_ui after we get the window size
            let vdom = VirtualDom::new_with_props(dioxus_app, props);

            // for sanity, force include a font with this library for testing for when linux isn't giving a proper font because of xyz
            // let mut font_ctx = FontContext::default();
            // // Use Blob::from(Vec<u8>) to avoid any `as _` coercion issues.
            // // The font data must be owned by the Blob for fontique to parse it.
            // font_ctx.collection.register_fonts(
            //     Blob::from(DEJAVU_SANS.to_vec()),
            //     None,
            // );
            let font_data: &'static [u8] = include_bytes!("../../assets/DejaVuSans.ttf");


            let mut font_ctx = FontContext::default();
            let families = font_ctx
                .collection
                .register_fonts(Blob::from(font_data.to_vec()), None);
            if let Some((family_id, _)) = families.first() {
                use parley::fontique::GenericFamily::*;
                for generic in [Serif, SansSerif, Monospace, Cursive, Fantasy, SystemUi] {
                    font_ctx
                        .collection
                        .set_generic_families(generic, std::iter::once(*family_id));
                }
            }

            
            let mut dioxus_doc = DioxusDocument::new(vdom, DocumentConfig {
                font_ctx: Some(font_ctx),
                ua_stylesheets: Some(vec![blitz_dom::DEFAULT_CSS.to_string()]),
                ..default()
            });
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
            documents.insert(entity, dioxus_doc);
        }

        // app.add_plugins(bevy_dioxus_interop_plugin);
        app.add_plugins(DioxusRenderPlugin);
        app.add_plugins(DioxusEventSyncPlugin);
        app.add_plugins(dioxus_signals_mirror_plugin);
        app.insert_non_send_resource(DioxusDocuments(documents));
    }
}
