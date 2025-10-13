use std::marker::PhantomData;

use bevy_ecs::component::Component;
use bevy_log::warn;
use crossbeam_channel::{Receiver, Sender};

use crate::BevyFetchBackup;

pub mod command;
pub mod hook;
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
    U: Component,
{
    value: Result<T, BevyFetchBackup>,
    write: Option<Sender<T>>,
    read: Option<Receiver<T>>,
    _marker: PhantomData<U>,
}

impl<T, U> BevyComponentSingleton<T, U>
where
    T: Component + Clone,
    U: Component,
{
    pub fn set_component(&self, value: T) {
        if let Some(write) = &self.write {
            let _ = write.send(value).inspect_err(|err| warn!("{:#}", err));
        }
    }
    pub fn read_component(&self) -> &Result<T, BevyFetchBackup> {
        &self.value
    }
}
