use dioxus::core::Element;
use std::{fmt::Debug, sync::Arc};

/// marks a struct as a Dioxus element. 
/// used to statically typed dioxus [`Element`]s
pub trait DioxusElementMarker: 'static + Sync + Send + Debug {
    //const ELEMENT_FUNCTION: fn() -> Element;
    fn element(&self) -> fn() -> Element;
}