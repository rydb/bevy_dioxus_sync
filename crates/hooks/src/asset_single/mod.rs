use std::{any::type_name, marker::PhantomData, ops::Deref};

use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_log::warn;
use crossbeam_channel::*;

use crate::BevyFetchBackup;

pub mod command;
pub mod hook;

pub struct AssetWithHandle<U>
where
//     T: Deref<Target = Handle<U>> + Component,
    U: Asset + Clone,
{
    handle: Handle<U>,
    asset: U,
    // new_type: PhantomData<T>,
}

// pub struct BevyWrappedAsset<U>
// where
//     U: Asset + Clone,
// {
//     value: Result<AssetWithHandle<U>, BevyFetchBackup>,
//     write: Option<Sender<AssetWithHandle<U>>,
//     read: Option<Receiver<AssetWithHandle<T, U>>>,
// }

// impl<T, U> BevyWrappedAsset<T, U>
// where
//     T: Deref<Target = Handle<U>> + Component,
//     U: Asset + Clone,
// {
//     pub fn set_asset(&self, value: U) {
//         let Ok(old_value) = &self.value else {
//             warn!("could not write asset as initial handle is not initialized");
//             return;
//         };
//         if let Some(send_channel) = &self.write {
//             let _ = send_channel
//             .send(AssetWithHandle {
//                 asset: value,
//                 handle: old_value.handle.clone(),
//                 new_type: PhantomData::default(),
//             })
//             .inspect_err(|err| warn!("{:#}", err));
//         } else {
//             warn!("no send channel for {:#}, skipping", type_name::<T>());
//             return
//         }
        
//     }
//     pub fn read_asset(&self) -> &Result<AssetWithHandle<T, U>, BevyFetchBackup> {
//         &self.value
//     }
// }
