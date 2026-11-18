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
use crate::slots::{CapturedPointer, FiveSlots};
use crate::worker::{VdomThreadRegistry, VdomWorker};
use crate::{DioxusUiQuad, SurfaceCursorHint, dioxus_ui, dioxus_uiProps};

/// Root layout drawn on a dioxus ui surface.
#[derive(Clone)]
pub enum LayoutSystem {
    /// A raw UI component as is.
    Raw(fn() -> Element),
    /// A [`FiveSlots`] ui layout.
    FiveSlots(FiveSlots),
}

impl PartialEq for LayoutSystem {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Raw(a), Self::Raw(b)) => std::ptr::fn_addr_eq(*a, *b),
            (Self::FiveSlots(a), Self::FiveSlots(b)) => a == b,
            (Self::Raw(_), Self::FiveSlots(_)) | (Self::FiveSlots(_), Self::Raw(_)) => false,
        }
    }
}

impl std::fmt::Debug for LayoutSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw(_) => f.write_str("Raw"),
            Self::FiveSlots(slots) => f.debug_tuple("FiveSlots").field(slots).finish(),
        }
    }
}

/// Ui for a given entity's surface
#[derive(Component, Clone, Default)]
#[require(DioxusUiQuad)]
pub struct DioxusUi {
    /// Layout drawn on the surface.
    pub layout: Option<LayoutSystem>,
}

/// Sends layout updates to a worker.
#[derive(Component)]
pub struct DioxusUiSender {
    sender: crossbeam_channel::Sender<Option<LayoutSystem>>,
}

/// Receives layout updates inside a worker.
#[derive(Clone)]
pub struct DioxusUiReceiver(pub crossbeam_channel::Receiver<Option<LayoutSystem>>);

/// Sends captured pointer messages to a worker.
#[derive(Component)]
pub struct DioxusPointerSender {
    sender: crossbeam_channel::Sender<CapturedPointer>,
}

impl DioxusPointerSender {
    /// Builds a sender for one document.
    pub fn new(sender: crossbeam_channel::Sender<CapturedPointer>) -> Self {
        Self { sender }
    }

    /// Sends one captured pointer message.
    pub fn send(&self, message: CapturedPointer) {
        let _ = self.sender.send(message);
    }
}

/// Receives captured pointer messages inside a worker.
#[derive(Clone)]
pub struct DioxusPointerReceiver(pub crossbeam_channel::Receiver<CapturedPointer>);

/// Receives the live pixel size of one dioxus ui surface.
#[derive(Clone)]
pub struct DioxusViewportReceiver(pub crossbeam_channel::Receiver<(u32, u32)>);

/// Marks a quad whose worker has started.
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
    cursor_hint: Res<SurfaceCursorHint>,
    mut commands: Commands,
) {
    for (e, quad) in quads {
        if registry.workers.contains_key(&e) {
            warn!(
                "document initialization requested for {} but worker already exists",
                e
            );
            continue;
        }

        let (layout_sender, layout_receiver) =
            crossbeam_channel::unbounded::<Option<LayoutSystem>>();
        let (viewport_tx, viewport_rx) = crossbeam_channel::unbounded::<(u32, u32)>();
        let (pointer_tx, pointer_rx) = crossbeam_channel::unbounded::<CapturedPointer>();
        let (proxy_sender, proxy_receiver) = crossbeam_channel::unbounded::<DioxusMessage>();

        if let Some(size) = quad.computed_wh {
            let _ = viewport_tx.try_send((size.x as u32, size.y as u32));
        }

        let vdom = VirtualDom::new_with_props(dioxus_ui, dioxus_uiProps { entity: e })
            .with_root_context(DioxusUiReceiver(layout_receiver))
            .with_root_context(DioxusViewportReceiver(viewport_rx))
            .with_root_context(DioxusPointerReceiver(pointer_rx))
            .with_root_context(command_queue_sender.clone())
            .with_root_context(cursor_hint.clone());

        let font_ctx = setup_fallback_font();

        let mut dioxus_doc = DioxusDocument::new(
            vdom,
            DocumentConfig {
                font_ctx: Some(font_ctx),
                ua_stylesheets: vec![blitz_dom::UserAgentStylesheet::parse(
                    blitz_dom::DEFAULT_CSS,
                )],
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
            if proxy_sender.send(DioxusMessage::Devserver(msg)).is_err() {
                warn!("devtools message dropped, vdom worker channel closed");
            }
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
            viewport_tx,
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

        commands.entity(e).insert(DioxusUiSender {
            sender: layout_sender,
        });
        commands
            .entity(e)
            .insert(DioxusPointerSender::new(pointer_tx));
        commands.entity(e).insert(InitializedVdom);
    }
}

/// Sync dioxus ui for a window with its latest layout.
pub(crate) fn sync_dioxus_ui_with_layouts(
    uis: Query<(&DioxusUi, &DioxusUiSender), Changed<DioxusUi>>,
) {
    for (ui, sender) in uis {
        let _ = sender
            .sender
            .send(ui.layout.clone())
            .inspect_err(|_err| error!("{_err}"));
    }
}
