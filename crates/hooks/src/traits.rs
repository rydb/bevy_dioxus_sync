//! TODO: Find better alternative to copying one trait bound across each generic map.
//!
//! can't get assocaited types to have "stricter impl's" then their oriignal definition, so this
//! is what we do to side step that...

use bevy_asset::{Asset, Handle};
use bevy_ecs::{component::Component, resource::Resource};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus::core::Element;
use std::{
    any::{type_name, Any, TypeId}, collections::HashMap, fmt::Debug, ops::Deref, sync::Arc
};

/// An untyped hashmap that resolved typed entries by their type id.
pub type ArcAnytypeMap = HashMap<TypeId, Arc<dyn Any + Send + Sync>>;

pub type BoxAnyTypeMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;


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
            .inspect_err(|err| warn!("could not downcast: {:#}: {:#?}", type_name::<T>(), err))
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

pub trait ErasedSubGenericComponentSingletonMap
where
    Self: TransparentWrapper<BoxAnyTypeMap> + Sized,
{
    type Generic<T: Component + Clone, U: Component>: Send + Sync + Clone + 'static;
    fn insert<T, U>(&mut self, value: Self::Generic<T, U>)
    where
        T: Component + Clone + Send + Sync + 'static,
        U: Component,
    {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T, U>(&mut self) -> Option<&mut Self::Generic<T, U>>
    where
        T: Component + Clone + Send + Sync + 'static,
        U: Component,
    {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get_mut(&TypeId::of::<T>())?;

        value.downcast_mut::<Self::Generic<T, U>>()
    }
    fn extend(&mut self, value: Self) {
        let map = TransparentWrapper::peel_mut(self);
        let value = TransparentWrapper::peel(value);
        map.extend(value);
    }
}

pub trait ErasedSubGenericAssetsMap
where
    Self: TransparentWrapper<BoxAnyTypeMap> + Sized,
{
    type Generic<T: Deref<Target = Handle<U>> + Component, U: Asset + Clone>: Send
        + Sync
        + Clone
        + 'static;
    fn insert<T, U>(&mut self, value: Self::Generic<T, U>)
    where
        T: Deref<Target = Handle<U>> + Component + 'static,
        U: Asset + Clone + Send + Sync + 'static,
    {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T, U>(&mut self) -> Option<&mut Self::Generic<T, U>>
    where
        T: Deref<Target = Handle<U>> + Component + 'static,
        U: Asset + Clone + Send + Sync + 'static,
    {
        let map = TransparentWrapper::peel_mut(self);

        let value = map.get_mut(&TypeId::of::<T>())?;

        value.downcast_mut::<Self::Generic<T, U>>()
    }
    fn extend(&mut self, value: Self) {
        let map = TransparentWrapper::peel_mut(self);
        let value = TransparentWrapper::peel(value);
        map.extend(value);
    }
}
