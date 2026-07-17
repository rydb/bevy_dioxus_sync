use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use bevy_dioxus_interop::DioxusMessage;
use bevy_dioxus_tracing::{error, warn};
use bevy_ecs::prelude::*;
use bevy_utils::default;
use blitz_dom::DocumentConfig;
#[cfg(target_os = "linux")]
use blitz_dom::FontContext;
use dioxus_bevy_signals::CommandQueueSender;
use dioxus_core::{Element, ScopeId, VirtualDom, provide_context};
use dioxus_native::DioxusDocument;
#[cfg(target_os = "linux")]
use vello::peniko::Blob;

use crate::net_provider::{BevyNetProvider, DioxusDocumentProxy};
use crate::worker::{VdomThreadRegistry, VdomWorker};
use crate::{DioxusUiQuad, dioxus_ui};

/// Panels on a dioxus ui quad surface
///
/// TODO: support multiple panel orientations (left, right, bottom, top, etc..)
#[derive(Component, Clone)]
#[require(DioxusUiQuad)]
pub struct DioxusPanels {
    pub panels: HashSet<fn() -> Element>,
}

impl Default for DioxusPanels {
    fn default() -> Self {
        Self {
            panels: Default::default(),
        }
    }
}

impl DioxusPanels {
    pub fn new(panels: Vec<fn() -> Element>) -> Self {
        let mut map = HashSet::default();
        for panel in panels {
            map.insert(panel);
        }
        Self { panels: map }
    }
}

#[derive(Component)]
pub struct DioxusPanelsSender {
    sender: crossbeam_channel::Sender<DioxusPanels>,
}

#[derive(Clone)]
pub struct DioxusPanelsReceiver(pub crossbeam_channel::Receiver<DioxusPanels>);

#[derive(Component)]
pub struct InitializedVdom;

/// On Linux, when blitz can't find a font, it will silently fail to render text.
/// This work around forcibly includes the below workaround font.
///
/// This work around also points blitz's default stylesheet fonts to this font.
#[cfg(target_os = "linux")]
fn setup_fallback_font() -> FontContext {
    let font_data: &'static [u8] = include_bytes!("../../../assets/JetBrainsMono-Medium.ttf");
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
    font_ctx
}

#[cfg(not(target_os = "linux"))]
fn setup_fallback_font() -> FontContext {
    FontContext::default()
}

/// Spawns a worker thread for each ui quad that has no VDOM yet.
pub(crate) fn initialize_vdoms(
    quads: Query<(Entity, &DioxusUiQuad), Without<InitializedVdom>>,
    mut registry: NonSendMut<VdomThreadRegistry>,
    command_queue_sender: Res<CommandQueueSender>,
    mut commands: Commands,
) {
    for (e, _quad) in quads {
        if registry.workers.contains_key(&e) {
            warn!(
                "document initialization requested for {} but worker already exists",
                e
            );
            continue;
        }

        let (panel_sender, panel_receiver) = crossbeam_channel::unbounded::<DioxusPanels>();
        let (proxy_sender, proxy_receiver) = crossbeam_channel::unbounded::<DioxusMessage>();

        let vdom = VirtualDom::new_with_props(dioxus_ui, ())
            .with_root_context(DioxusPanelsReceiver(panel_receiver))
            .with_root_context(command_queue_sender.clone());

        let font_ctx = setup_fallback_font();

        let mut dioxus_doc = DioxusDocument::new(
            vdom,
            DocumentConfig {
                font_ctx: Some(font_ctx),
                ua_stylesheets: Some(vec![blitz_dom::DEFAULT_CSS.to_string()]),
                ..default()
            },
        );

        let net_provider = BevyNetProvider::shared();
        dioxus_doc.inner.borrow_mut().set_net_provider(net_provider);

        let proxy = Rc::new(DioxusDocumentProxy::new(proxy_sender.clone()));
        dioxus_doc.vdom.in_scope(ScopeId::ROOT, move || {
            provide_context(proxy as Rc<dyn dioxus_document::Document>);
        });

        dioxus_devtools::connect(move |msg| {
            proxy_sender.send(DioxusMessage::Devserver(msg)).unwrap()
        });

        dioxus_doc.initial_build();

        // Set up channels for worker communication.
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let (result_tx, result_rx) = crossbeam_channel::unbounded();
        let (input_tx, input_rx) = crossbeam_channel::unbounded();
        let waker_flag = Arc::new(AtomicBool::new(false));

        let handle = VdomWorker::spawn(
            e,
            dioxus_doc,
            proxy_receiver,
            cmd_rx,
            result_tx,
            input_rx,
            waker_flag.clone(),
        );

        let worker = VdomWorker {
            cmd_tx,
            result_rx,
            input_tx,
            waker_flag,
            thread: Some(handle),
        };
        registry.workers.insert(e, worker);

        commands.entity(e).insert(DioxusPanelsSender {
            sender: panel_sender,
        });
        commands.entity(e).insert(InitializedVdom);
    }
}

/// sync dioxus ui for a window with its latest panels
pub(crate) fn sync_dioxus_ui_with_panels(
    // mut panels: Query<(&DioxusPanels, &DioxusPanelsSender)> Changed<DioxusPanels>>
    panels: Query<(&DioxusPanels, &DioxusPanelsSender), Changed<DioxusPanels>>,
) {
    for (panels, sender) in panels {
        let _ = sender
            .sender
            .send(panels.clone())
            .inspect_err(|_err| error!("{_err}"));
    }
}
