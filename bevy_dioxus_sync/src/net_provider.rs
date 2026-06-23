use std::sync::Arc;

use bevy_dioxus_render::{DioxusMessage, HeadElement};
use bevy_dioxus_tracing::warn;
use blitz_traits::net::{NetHandler, NetProvider};
use bytes::Bytes;
use crossbeam_channel::Sender;
use data_url::DataUrl;
use dioxus_document::{LinkProps, MetaProps, NoOpDocument, ScriptProps, StyleProps};

pub struct BevyNetProvider;

impl BevyNetProvider {
    pub(crate) fn shared() -> Arc<dyn NetProvider> {
        Arc::new(Self) as _
    }
}

impl NetProvider for BevyNetProvider {
    fn fetch(
        &self,
        _doc_id: usize,
        request: blitz_traits::net::Request,
        handler: Box<dyn NetHandler>,
    ) {
        match request.url.scheme() {
            // Load Dioxus assets
            "dioxus" => match dioxus_asset_resolver::native::serve_asset(request.url.path()) {
                Ok(res) => handler.bytes(request.url.to_string(), res.into_body().into()),
                Err(err) => {
                    warn!("{err}");

                }
            },
            // Decode data URIs
            "data" => {
                let Ok(data_url) = DataUrl::process(request.url.as_str()) else {
                    return;
                };
                let Ok(decoded) = data_url.decode_to_vec() else {
                    return;
                };
                let bytes = Bytes::from(decoded.0);
                handler.bytes(request.url.to_string(), bytes);
            }
            // TODO: support http requests
            _ => {
                warn!("unsupported scheme detected for {_doc_id} for request {:#?}", request);
            }
        }
    }
}

pub struct DioxusDocumentProxy {
    sender: Sender<DioxusMessage>,
}

impl DioxusDocumentProxy {
    pub(crate) fn new(sender: Sender<DioxusMessage>) -> Self {
        Self { sender }
    }
}

impl dioxus_document::Document for DioxusDocumentProxy {
    fn eval(&self, _js: String) -> dioxus_document::Eval {
        NoOpDocument.eval(_js)
    }

    fn create_head_element(
        &self,
        name: &str,
        attributes: &[(&str, String)],
        contents: Option<String>,
    ) {
        self.sender
            .send(DioxusMessage::CreateHeadElement(HeadElement {
                name: name.to_string(),
                attributes: attributes
                    .iter()
                    .map(|(name, value)| (name.to_string(), value.clone()))
                    .collect(),
                contents,
            }))
            .unwrap();
    }

    fn set_title(&self, title: String) {
        self.create_head_element("title", &[], Some(title));
    }

    fn create_meta(&self, props: MetaProps) {
        let attributes = props.attributes();
        self.create_head_element("meta", &attributes, None);
    }

    fn create_script(&self, props: ScriptProps) {
        let attributes = props.attributes();
        self.create_head_element("script", &attributes, props.script_contents().ok());
    }

    fn create_style(&self, props: StyleProps) {
        let attributes = props.attributes();
        self.create_head_element("style", &attributes, props.style_contents().ok());
    }

    fn create_link(&self, props: LinkProps) {
        let attributes = props.attributes();
        self.create_head_element("link", &attributes, None);
    }

    fn create_head_component(&self) -> bool {
        true
    }
}
