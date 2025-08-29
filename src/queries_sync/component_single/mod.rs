use std::marker::PhantomData;

use bevy_ecs::component::Component;
use bevy_log::warn;
use crossbeam_channel::{Receiver, Sender};

use crate::queries_sync::asset_single::AssetWithHandle;

pub mod hook;
pub mod command;
// pub struct BevyComponent<T> 
//     where
//         T: Component
// {
//     value: Option<T>,
//     write: Sender<T>,
//     read: Receiver<T>,
// }


// pub struct BevyComponent<T: Component>(pub T);

pub struct BevyComponentSingleton<T, U> 
    where
        T: Component + Clone,
        U: Component
{
    value: Option<T>,
    write: Sender<T>,
    read: Receiver<T>,
    _marker: PhantomData<U>
}

impl<T, U> BevyComponentSingleton<T, U> 
    where
        T: Component + Clone,
        U: Component
{
    pub fn set_component(&self, value: T) {
        let _ = self.write.send(value).inspect_err(|err| warn!("{:#}",err));
    }
    pub fn read_component(&self) -> &Option<T>{
        &self.value
    }
}
