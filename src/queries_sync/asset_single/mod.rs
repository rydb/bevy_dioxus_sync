use std::{marker::PhantomData, ops::Deref};

use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use crossbeam_channel::*;
use dioxus::signals::Signal;

use crate::BoxAnyTypeMap;

pub mod hook;
pub mod command;

pub struct AssetWithHandle<T, U> 
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone
{
    handle: Handle<U>,
    asset: U,
    new_type: PhantomData<T>,
}

pub struct BevyWrappedAsset<T, U> 
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone
{
    value: Option<AssetWithHandle<T, U>>,
    write: Sender<AssetWithHandle<T, U>>,
    read: Receiver<AssetWithHandle<T, U>>,
}

impl<T, U> BevyWrappedAsset<T, U> 
    where
        T: Deref<Target = Handle<U>> + Component,
        U: Asset + Clone
{
    pub fn write_asset(&self, value: U) {
        let Some(old_value) = &self.value else {
            warn!("could not write asset as initial handle is not initialized");
            return
        };
        let _ = self.write.send(AssetWithHandle {
            asset: value,
            handle: old_value.handle.clone(),
            new_type: PhantomData::default()
        }).inspect_err(|err| warn!("{:#}",err));
    }
    pub fn read_asset(&self) -> &Option<AssetWithHandle<T, U>>{
        &self.value
    }
}