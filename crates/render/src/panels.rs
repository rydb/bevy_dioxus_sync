use std::{collections::HashSet, rc::Rc};

use bevy_app::{Plugin, PreUpdate, Update};
use bevy_dioxus_interop::{DioxusDocumentInfo, DioxusDocuments};
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

use crate::{DioxusMessage, DioxusUiQuad, dioxus_ui, net_provider::{BevyNetProvider, DioxusDocumentProxy}};

/// panels inside of a dioxus window
#[derive(Component, Clone)]
#[require(DioxusUiQuad)]
pub struct DioxusPanels {
    pub panels: HashSet<fn() -> Element>, 
}

impl Default for DioxusPanels {
    fn default() -> Self {
        Self { panels: Default::default() }
    }
}

impl DioxusPanels {
    pub fn new(panels: Vec<fn() -> Element>) -> Self {
        let mut map = HashSet::default();
        for panel in panels {
            map.insert(panel);
        }
        Self {
            panels: map
        }
    }
}


#[derive(Component)]
pub struct DioxusPanelsSender {
    sender: crossbeam_channel::Sender<DioxusPanels>
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

/// initialize vdoms for UiQuads without them
fn initialize_vdoms(
    quads: Query<(Entity, &DioxusUiQuad), Without<InitializedVdom>>,
    mut documents: NonSendMut<DioxusDocuments>,
    command_queue_sender: Res<CommandQueueSender>,
    mut commands: Commands
) {
    for (e, _quad) in quads {
        if documents.0.contains_key(&e)  == true {
            warn!("document initialization requested for {} but initialized document already exists?", e);
            return;
        }

        
        let (panel_sender, panel_receiver) = crossbeam_channel::unbounded::<DioxusPanels>();
        let (proxy_sender, proxy_receiver) = crossbeam_channel::unbounded::<DioxusMessage>();
        // let props = DioxusUiProps {
        //     panel_receiver: DioxusPanelsReceiver(panel_receiver),
        //     command_queue_sender: command_queue_sender.clone(),
        // };

        let vdom = VirtualDom::new_with_props(dioxus_ui, ())
        // .with_root_context(props)
        // .with;
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

        // Setup NetProvider. Required to load any assets inside of dioxus.
        // If something isn't loading, check:
        // - That the net provider is configured for your asset type
        // - That its not a blitz rendering issue: https://github.com/DioxusLabs/blitz/issues/119
        // - That its not a font rendering issue (fallback font on linux addresses this, but for other OS(s) this may be an issue)
        let net_provider = BevyNetProvider::shared();

        dioxus_doc.inner.borrow_mut().set_net_provider(net_provider);

        // Setup DocumentProxy to process CreateHeadElement messages
        let proxy = Rc::new(DioxusDocumentProxy::new(proxy_sender.clone()));
        dioxus_doc.vdom.in_scope(ScopeId::ROOT, move || {
            provide_context(proxy as Rc<dyn dioxus_document::Document>);
        });

        dioxus_devtools::connect(move |msg| proxy_sender.send(DioxusMessage::Devserver(msg)).unwrap());

        dioxus_doc.initial_build();

        let info = DioxusDocumentInfo {
            document: dioxus_doc,
            messages_recv: proxy_receiver,
        };
        documents.0.insert(e, info);

        commands.entity(e).insert(DioxusPanelsSender {sender: panel_sender});
        commands.entity(e).insert(InitializedVdom);
    }
}

/// sync dioxus ui for a window with its latest panels
fn sync_dioxus_ui_with_panels(
    // mut panels: Query<(&DioxusPanels, &DioxusPanelsSender)> Changed<DioxusPanels>>
    panels: Query<(&DioxusPanels, &DioxusPanelsSender), Changed<DioxusPanels>>,
) {
    for (panels, sender)  in panels {
        let _ = sender.sender.send(panels.clone()).inspect_err(|_err| error!("{_err}"));
    }
}

/// systems for setting up support for panels in dioxus ui
pub struct DioxusUiPanelsPlugin;

impl Plugin for DioxusUiPanelsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app
        .add_systems(PreUpdate, initialize_vdoms)
        .add_systems(Update, sync_dioxus_ui_with_panels)
        ;
    }
}