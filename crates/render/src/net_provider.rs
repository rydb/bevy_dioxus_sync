use std::ffi::OsStr;
use std::path::Path;
use std::sync::Arc;

use bevy_dioxus_interop::HeadElement;
use bevy_dioxus_tracing::warn;
use blitz_traits::net::{NetHandler, NetProvider};
use bytes::Bytes;
use crossbeam_channel::Sender;
use data_url::DataUrl;
use dioxus_document::{LinkProps, MetaProps, NoOpDocument, ScriptProps, StyleProps};
use manganis_core::{create_module_hash, transform_css};
use percent_encoding::percent_decode_str;

use crate::DioxusMessage;

pub struct BevyNetProvider;

/// Appends scoped css module rules to a stylesheet read from disk.
///
/// Asset staging scopes css modules while bundling. Builds without the bundler serve the source
/// stylesheet, so the scoped rules are appended to the source rules.
fn scope_css_module(encoded_path: &str, body: Bytes) -> Bytes {
    let Ok(decoded_path) = percent_decode_str(encoded_path).decode_utf8() else {
        return body;
    };
    let path = Path::new(decoded_path.as_ref());
    if path.extension() != Some(OsStr::new("css")) || !path.is_file() {
        return body;
    }
    let Ok(css) = std::str::from_utf8(&body) else {
        return body;
    };
    let Ok(scoped) = transform_css(css, &create_module_hash(path)) else {
        return body;
    };
    let mut stylesheet = String::with_capacity(css.len() + scoped.len() + 1);
    stylesheet.push_str(css);
    stylesheet.push('\n');
    stylesheet.push_str(&scoped);
    Bytes::from(stylesheet)
}

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
                Ok(res) => {
                    let body = scope_css_module(request.url.path(), res.into_body().into());
                    handler.bytes(request.url.to_string(), body);
                }
                Err(_err) => {
                    warn!("{_err}");
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
                warn!(
                    "unsupported scheme detected for {_doc_id} for request {:#?}",
                    request
                );
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

#[cfg(test)]
mod tests {
    use super::*;

    fn slots_css_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("slots")
            .join("style.css")
    }

    #[test]
    fn source_stylesheet_gains_scoped_rules() {
        let path = slots_css_path();
        let source = std::fs::read_to_string(&path).unwrap();
        let request_path = path.to_str().unwrap();
        let response = dioxus_asset_resolver::native::serve_asset(request_path).unwrap();
        let body = scope_css_module(request_path, response.into_body().into());
        let stylesheet = std::str::from_utf8(&body).unwrap();
        assert!(stylesheet.starts_with(&source));
        let scoped_class = format!(".slot-title-{}", create_module_hash(&path));
        assert!(stylesheet.contains(&scoped_class));
    }

    #[test]
    fn staged_stylesheet_is_served_unchanged() {
        let body = Bytes::from_static(b".slot-title { color: red; }");
        let served = scope_css_module("/assets/slots-2fb3dca5.css", body.clone());
        assert_eq!(served, body);
    }
}
