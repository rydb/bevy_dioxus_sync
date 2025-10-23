use bevy_dioxus_render::{DioxusMessage, HeadElement};
use bytes::Bytes;
use crossbeam_channel::Sender;
use data_url::DataUrl;
use dioxus_document::{LinkProps, MetaProps, NoOpDocument, ScriptProps, StyleProps};
use std::sync::Arc;



pub mod panels;
pub mod plugins;
pub(crate) mod systems;
pub mod ui;
struct BevyNetCallback {
    sender: Sender<DioxusMessage>,
}

use blitz_dom::net::Resource as BlitzResource;
use blitz_traits::net::{NetCallback, NetHandler, NetProvider};

impl NetCallback<BlitzResource> for BevyNetCallback {
    fn call(&self, _doc_id: usize, result: core::result::Result<BlitzResource, Option<String>>) {
        if let Ok(res) = result {
            self.sender.send(DioxusMessage::ResourceLoad(res)).unwrap();
        }
    }
}

pub struct BevyNetProvider {
    callback: Arc<dyn NetCallback<BlitzResource> + 'static>,
}
impl BevyNetProvider {
    fn shared(sender: Sender<DioxusMessage>) -> Arc<dyn NetProvider<BlitzResource>> {
        Arc::new(Self::new(sender)) as _
    }

    fn new(sender: Sender<DioxusMessage>) -> Self {
        Self {
            callback: Arc::new(BevyNetCallback { sender }) as _,
        }
    }
}

impl NetProvider<BlitzResource> for BevyNetProvider {
    fn fetch(
        &self,
        doc_id: usize,
        request: blitz_traits::net::Request,
        handler: Box<dyn NetHandler<BlitzResource>>,
    ) {
        match request.url.scheme() {
            // Load Dioxus assets
            "dioxus" => match dioxus_asset_resolver::native::serve_asset(request.url.path()) {
                Ok(res) => handler.bytes(doc_id, res.into_body().into(), self.callback.clone()),
                Err(_) => {
                    self.callback.call(
                        doc_id,
                        Err(Some(String::from("Error loading Dioxus asset"))),
                    );
                }
            },
            // Decode data URIs
            "data" => {
                let Ok(data_url) = DataUrl::process(request.url.as_str()) else {
                    self.callback
                        .call(doc_id, Err(Some(String::from("Failed to parse data uri"))));
                    return;
                };
                let Ok(decoded) = data_url.decode_to_vec() else {
                    self.callback
                        .call(doc_id, Err(Some(String::from("Failed to decode data uri"))));
                    return;
                };
                let bytes = Bytes::from(decoded.0);
                handler.bytes(doc_id, bytes, Arc::clone(&self.callback));
            }
            // TODO: support http requests
            _ => {
                self.callback
                    .call(doc_id, Err(Some(String::from("UnsupportedScheme"))));
            }
        }
    }
}

pub struct DioxusDocumentProxy {
    sender: Sender<DioxusMessage>,
}

impl DioxusDocumentProxy {
    fn new(sender: Sender<DioxusMessage>) -> Self {
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
