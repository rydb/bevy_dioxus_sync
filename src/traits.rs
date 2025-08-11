use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus::core::Element;
use std::{any::{type_name, Any, TypeId}, fmt::Debug, sync::Arc};

use crate::AnytypeMap;

/// marks a struct as a Dioxus element. 
/// used to statically typed dioxus [`Element`]s
pub trait DioxusElementMarker: 'static + Sync + Send + Debug {
    //const ELEMENT_FUNCTION: fn() -> Element;
    fn element(&self) -> Element;
}

pub trait ErasedSubGenericMap
    where
        Self: TransparentWrapper<AnytypeMap> + Sized,
{
    type Generic<T: Send + Sync + 'static>: Send + Sync + 'static;
    fn insert<T: Send + Sync + 'static>(&mut self, value: Self::Generic<T>)
        where
            // Self::Generic<T>: From<T>,
    {   
        let map = TransparentWrapper::peel_mut(self);
        let erased: Arc<dyn Any + Send + Sync> = Arc::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T: Send + Sync + 'static>(&mut self) -> Option<Arc<Self::Generic<T>>> {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get(&TypeId::of::<T>())?.clone();
        value.downcast::<Self::Generic<T>>().inspect_err(|err| warn!("could not downcast: {:#}", type_name::<T>())).ok()
    }
    fn extend(&mut self, value: Self) {
        let map = TransparentWrapper::peel_mut(self);
        let value = TransparentWrapper::peel(value);
        map.extend(value);
    }
}