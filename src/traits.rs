use bevy_ecs::resource::Resource;
use bevy_log::warn;
use bevy_utils::default;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use dioxus::core::Element;
use std::{any::{type_name, Any, TypeId}, default, fmt::Debug, mem, sync::Arc};

use crate::{AnyType, ArcAnytypeMap, BoxAnyTypeMap, BoxVal};

/// marks a struct as a Dioxus element. 
/// used to statically typed dioxus [`Element`]s
pub trait DioxusElementMarker: 'static + Sync + Send + Debug {
    //const ELEMENT_FUNCTION: fn() -> Element;
    fn element(&self) -> Element;
}

pub trait ErasedSubGeneriResourcecMap
    where
        Self: TransparentWrapper<BoxAnyTypeMap> + Sized,
{
    type Generic<T: Clone + Resource + Send + Sync>: Send + Sync + Clone + 'static;
    fn insert<T: Clone + Resource + Send + Sync + 'static>(&mut self, value: Self::Generic<T>){   
        let map = TransparentWrapper::peel_mut(self);
        let erased= Box::new(value);
        map.insert(TypeId::of::<T>(), erased);
    }

    fn get<T: Clone + Resource + Send + Sync + 'static>(&mut self) -> Option<&mut Self::Generic<T>> {
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
    fn insert<T: Send + Sync + 'static>(&mut self, value: Self::Generic<T>){   
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

/// Anytype that that is a `Generic<T>` mapped by its `T`. E.G: `Sender<T>` would be indexed by `T`
pub trait ErasedSubGeneric
    where
        Self: TransparentWrapper<AnyType> + Sized
{
    type Generic<T: Send + Sync + 'static>: Send + Sync + 'static;

    fn new<T: Send + Sync + 'static>(value: Self::Generic<T>) -> Self{
        let any_type = (TypeId::of::<T>(), BoxVal::new(value));
        let new_self = Self::wrap(any_type);
        new_self
    } 
    fn get<T: Send + Sync + 'static>(&mut self) -> Box<Self::Generic<T>> 
    {
        let (a,  box_ptr) = TransparentWrapper::peel_mut(self);

        let ptr = box_ptr.take();
        
        let value = ptr.downcast::<Self::Generic<T>>().unwrap();
        value
    }
    // fn get_untyped(self, id_check: TypeId) -> BoxSync {
    //     let (type_id, data) = TransparentWrapper::peel(self);
    //     if (type_id)
    // }
}

