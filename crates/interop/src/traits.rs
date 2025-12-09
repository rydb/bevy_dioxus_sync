use crate::signals::CrossDomSignal;
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use std::any::type_name;
use std::fmt::Debug;
use std::{any::Any, collections::HashMap, hash::Hash};

// pub type BoxGenericTypeMap<Index> = HashMap<Index, Box<dyn Any + Send + Sync>>;

pub type SyncBox = Box<dyn Any + Send + Sync>;

// // /// A struct that holds a type-erased read-clone of a bevy clone, and hashmap of all read-signals that map to it.
// pub struct ErasedSyncReadSignalMap(HashMap<Entity, SyncBox>);

// impl ErasedSyncReadSignalMap {
//     pub fn set_value<T: Clone + Send + Sync + 'static>(&mut self, value: T) {
//         // let new_signal: CrossDomSignal<T> = CrossDomSignal::<T>::new(value);

//         for (e, signal) in &mut self.subscribers {
//             warn!("subscrubing value {}, for panel on {} ", type_name::<T>(), e);
//             let signal = signal.downcast_mut::<CrossDomSignal<T>>().unwrap();

//             let result = signal.set(value);
//             match result {
//                 Ok(_) => {},
//                 Err(err) => warn!("COULD NOT POINT TO NEW VALUE FOR {}, BORROW ERROR: {}", type_name::<T>(), err),
//             }
//         }
//         let erased = Box::new(new_signal);
//         self.owner = erased;
//     }
//     pub fn new<T: Send + Sync + 'static>(value: T) -> Self {
//         Self {
//             owner: Box::new(Self::new_signal(value)),
//             subscribers: HashMap::new()
//         }

//     }
// }

pub type BoxGenericDomTypeMap<Index> = HashMap<Index, ErasedSignal>;
// pub type SignalTypeDomMap<Index> = HashMap<Index, HashMap<Entity, Box<dyn Any + Send + Sync>>>;
// pub type SignalErasedMapValue<T, Index, AdditionalInfo> =
//     CrossDomSignal<BevyValue<T, Index, AdditionalInfo>>;

// pub type ErasedSignalValue<T, Index, AdditionalInfo> =
//     CrossDomSignal<BevyValue<T, Index, AdditionalInfo>>;

pub type ErasedSignalValue<T> = CrossDomSignal<T>;

// pub type ReadSignalErasedMapValue<T, Index, AdditionalInfo> = ReadSignal<BevyValue<T, Index, AdditionalInfo>, SyncStorage>;

// pub trait SignalsErasedMap
// where
//     Self: TransparentWrapper<BoxGenericTypeMap<Self::Index>> + Sized,
// {
//     /// What type a signal is indexed by
//     type Index: Debug + Hash + Eq + Clone + Send + Sync + 'static;

//     /// additional to be sent to/from bevy.
//     type AdditionalInfo: Send + Sync + 'static + Clone;
//     fn insert_typed<T: Clone + Send + Sync + 'static>(
//         &mut self,
//         value: SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>,
//         index: Self::Index,
//     ) {
//         let map = TransparentWrapper::peel_mut(self);
//         let erased = Box::new(value);
//         map.insert(index, erased);
//     }

//     fn get_typed<T: Clone + Send + Sync + 'static>(
//         &mut self,
//         index: &Self::Index,
//     ) -> Option<&mut SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>> {
//         let map = TransparentWrapper::peel_mut(self);

//         let value = map.get_mut(&index)?;

//         value.downcast_mut::<SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>>()
//     }
// }

// pub trait ReadSignalsErasedMap
// where
//     Self: TransparentWrapper<BoxGenericDomTypeMap<Self::Index>> + Sized,
// {
//     /// What type a signal is indexed by
//     type Index: Debug + Hash + Eq + Clone + Send + Sync + 'static;

//     /// additional to be sent to/from bevy.
//     type AdditionalInfo: Send + Sync + 'static + Clone;
//     fn insert_typed<T: Clone + Send + Sync + 'static>(
//         &mut self,
//         value: T,
//         index: Self::Index,
//     ) {
//         let map = TransparentWrapper::peel_mut(self);
//         let erased = ErasedSyncReadSignalMap::new(value);
//         map.insert(index, erased);
//     }

//     fn get_typed<T: Send + Sync + 'static>(
//         &mut self,
//         index: &Self::Index,
//     ) -> Option<&mut ErasedSyncReadSignalMap>
//     //Option<&mut SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>>
//     {
//         let map = TransparentWrapper::peel_mut(self);

//         return map.get_mut(&index);

//         // value
//         //value.owner.downcast_mut::<SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>>()
//     }
// }

pub type ErasedSignal = Box<dyn Any + Send + Sync>;

pub trait CrossDomSignalErasedMap
where
    Self: TransparentWrapper<BoxGenericDomTypeMap<Self::Index>> + Sized,
{
    /// What type a signal is indexed by
    type Index: Debug + Hash + Eq + Clone + Send + Sync + 'static;

    /// additional to be sent to/from bevy.
    type AdditionalInfo: Send + Sync + 'static + Clone;
    fn insert_value<T: Clone + Send + Sync + 'static>(&mut self, value: T, index: Self::Index) {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(ErasedSignalValue::new(value));
        map.insert(index, erased);
    }

    fn insert_signal<T: Clone + Send + Sync + 'static>(&mut self, value: T, index: Self::Index) {
        let map = TransparentWrapper::peel_mut(self);
        let erased = Box::new(value);
        map.insert(index, erased);
    }
    fn get_typed<T: Clone + Send + Sync + 'static>(
        &mut self,
        index: &Self::Index,
    ) -> Option<&mut ErasedSignalValue<T>>
//Option<&mut SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>>
    {
        let map: &mut HashMap<
            <Self as CrossDomSignalErasedMap>::Index,
            Box<dyn Any + Send + Sync + 'static>,
        > = TransparentWrapper::peel_mut(self);

        match map.get_mut(&index) {
            Some(value) => value.downcast_mut::<ErasedSignalValue<T>>(),
            None => {
                warn!("could not downcast signal for {}", type_name::<T>());
                None
            }
        }

        // value
        //value.owner.downcast_mut::<SignalErasedMapValue<T, Self::Index, Self::AdditionalInfo>>()
    }
}
