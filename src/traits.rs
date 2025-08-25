//! TODO: Find better alternative to copying one trait bound across each generic map.
//! 
//! can't get assocaited types to have "stricter impl's" then their oriignal definition, so this
//! is what we do to side step that...

use bevy_ecs::{component::Component, resource::Resource};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus::core::Element;
use std::{
    any::{Any, TypeId, type_name},
    fmt::Debug,
    sync::Arc,
};

use crate::{ArcAnytypeMap, BoxAnyTypeMap};

/// marks a struct as a Dioxus element.
/// used to statically typed dioxus [`Element`]s
pub trait DioxusElementMarker: 'static + Sync + Send + Debug {
    //const ELEMENT_FUNCTION: fn() -> Element;
    fn element(&self) -> Element;
}

pub trait ErasedSubGenericResourcecMap
where
    Self: TransparentWrapper<BoxAnyTypeMap> + Sized,
{
    type Generic<T: Clone + Resource + Send + Sync>: Send + Sync + Clone + 'static;
    fn insert<T: Clone + Resource + Send + Sync + 'static>(&mut self, value: Self::Generic<T>) {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T: Clone + Resource + Send + Sync + 'static>(
        &mut self,
    ) -> Option<&mut Self::Generic<T>> {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get_mut(&TypeId::of::<T>())?;

        value.downcast_mut::<Self::Generic<T>>()
    }
    fn extend(&mut self, value: Self) {
        let map = TransparentWrapper::peel_mut(self);
        let value = TransparentWrapper::peel(value);
        map.extend(value);
    }
}

pub trait ErasedSubGenericMap
where
    Self: TransparentWrapper<ArcAnytypeMap> + Sized,
{
    type Generic<T: Send + Sync + 'static>: Send + Sync + 'static;
    fn insert<T: Send + Sync + 'static>(&mut self, value: Self::Generic<T>) {
        let map = TransparentWrapper::peel_mut(self);
        let erased: Arc<dyn Any + Send + Sync> = Arc::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T: Send + Sync + 'static>(&mut self) -> Option<Arc<Self::Generic<T>>> {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get(&TypeId::of::<T>())?.clone();
        value
            .downcast::<Self::Generic<T>>()
            .inspect_err(|err| warn!("could not downcast: {:#}", type_name::<T>()))
            .ok()
    }
    fn extend(&mut self, value: Self) {
        let map = TransparentWrapper::peel_mut(self);
        let value = TransparentWrapper::peel(value);
        map.extend(value);
    }
}


pub trait ErasedSubGenericComponentsMap
where
    Self: TransparentWrapper<BoxAnyTypeMap> + Sized,
{
    type Generic<T: Clone + Component + Send + Sync>: Send + Sync + Clone + 'static;
    fn insert<T: Clone + Component + Send + Sync + 'static>(&mut self, value: Self::Generic<T>) {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T: Clone + Component + Send + Sync + 'static>(
        &mut self,
    ) -> Option<&mut Self::Generic<T>> {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get_mut(&TypeId::of::<T>())?;

        value.downcast_mut::<Self::Generic<T>>()
    }
    fn extend(&mut self, value: Self) {
        let map = TransparentWrapper::peel_mut(self);
        let value = TransparentWrapper::peel(value);
        map.extend(value);
    }
}
