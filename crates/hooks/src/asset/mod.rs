use std::any::{TypeId, type_name};
use std::collections::{HashMap, HashSet};

use bevy_app::Update;
use bevy_asset::{Asset, Assets, Handle};
use bevy_asset::prelude::*;
use bevy_dioxus_interop::add_systems_through_world;
use bevy_dioxus_interop::signals::CrossDomSignal;
use bevy_ecs::storage::ResourceData;
use bevy_ecs::prelude::*;
use bevy_ecs::{change_detection::DetectChanges, resource::Resource, system::{Command, ResMut}};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus_hooks::Dependency;
use std::fmt::Debug;

use crate::use_bevy_value;

// pub fn use_bevy_asset<T: Asset>(handle: Handle<T>) -> BevyAssetClone<T> {
//     use_bevy
// }

#[derive(Resource, Default)]
pub struct CheckedAssetTypes(HashSet<TypeId>);

#[derive(TransparentWrapper, Clone)]
#[repr(transparent)]
pub struct RequestBevyAssets<T: Asset>(BevyAssetsHolder<T>);

impl<T: Asset + Clone> Command for RequestBevyAssets<T> {
    fn apply(self, world: &mut bevy_ecs::world::World) -> () {
        
        match world.get_resource::<BevyAssetsHolder<T>>() {
            Some(n) => self.0.0.pnt_to(n.0.get_ptr().unwrap()),
            None => {
                // let x = self.0.0
                self.0.0.initialize(HashMap::new());
                world.insert_resource(self.0.clone());
            }
        }
        let mut checked_asset_types = world.get_resource_or_init::<CheckedAssetTypes>();

        if checked_asset_types.0.contains(&TypeId::of::<T>()) == false {
            checked_asset_types.0.insert(TypeId::of::<T>());
            add_systems_through_world(world, Update, 
                manage_asset_status::<T>
            );
        }
        
    }
}

pub fn manage_asset_status<T: Clone + Asset>(
    mut assets: ResMut<Assets<T>>,
    assets_clone: ResMut<BevyAssetsHolder<T>>,
    asset_events: ResMut<Messages<AssetEvent<T>>>,
) { 
    let Ok(dioxus_asset_state) = assets_clone.0.get().inspect_err(|err| warn!("{err}")) else {
        return
    };
    let mut assets_cursor = asset_events.get_cursor();
    let mut change_event_asset_ids = Vec::new();
    
    let changed_assets = assets_cursor.read(&asset_events);

    for asset_events in changed_assets {
        match asset_events {
            AssetEvent::Modified { id } => change_event_asset_ids.push(id),
            _ => {}

        }
    }

    for (handle, signal) in dioxus_asset_state.as_ref() {
        
        let asset = match assets.get_mut(handle) {
            Some(n) => {
                n
            },
            None => todo!("implement behaviour for when asset is removed but handles for it still exist in dioxus."),
        };
        // bevy side asset updates take priority over dioxus side updates, so their update over-rides a dioxus update by moving first if they are both changed at the same time.
        if change_event_asset_ids.contains(&&handle.id()) {
            let _ = signal.set(BevyAssetClone::loaded(asset.clone())).inspect_err(|err| warn!("{err}"));
            
            continue;
        } else {
            let Ok(signal_value) = signal.get().inspect_err(|err| warn!("{err}")) else  {
                continue
            };
            match signal_value.as_ref() {
                BevyAssetClone::Loading(_handle) => {
                    let _ = signal.set(BevyAssetClone::loaded(asset.clone())).inspect_err(|err| warn!("{err}"));
                },
                BevyAssetClone::Loaded(current_value) => {
                    // todo!();
                    if current_value.dioxus_changed == true {
                        warn!("setting new asset value..");

                        *asset = current_value.value.clone();
                    }
                } ,
            }
        }
    }
} 


// pub type BevyAssetsValue<T> = HashMap<Handle<T>, CrossDomSignal<BevyAssetClone<T>>>;

// pub struct AssetHolderMap(HashMap<Handle<T>>, CrossDomSignal<BevyAs>)

pub type BevyAssetsValue<T> = HashMap<Handle<T>, CrossDomSignal<BevyAssetClone<T>>>;

/// For tracking 
#[derive(Clone,Debug)]
pub struct ChangeTracked<T> {
    value: T,
    dioxus_changed: bool
}

impl<T> ChangeTracked<T> {
    pub fn get(&self) -> &T {
        &self.value
    }
}

#[derive(TransparentWrapper, Resource, Clone)]
#[repr(transparent)]
pub struct BevyAssetsHolder<T: Asset>(CrossDomSignal<BevyAssetsValue<T>>);


#[derive(Clone, Debug)]
pub enum BevyAssetClone<T: Asset> {
    Loading(Handle<T>),
    Loaded(ChangeTracked<T>)
}

#[derive(Debug)]
pub enum AssetSetError {
    Uninitialized
}

#[derive(Debug)]
pub enum AssetGetError {
    Uninitialized,
    DoesntExist,
}

impl<T: Asset> BevyAssetClone<T> {
    /// set value of bevy asset
    pub fn set(&mut self, value: T) -> Result<(), AssetSetError> {
        match self {
            BevyAssetClone::Loading(_handle) => Err({
                warn!("could not set asset for {}. Asset not initialized ", type_name::<T>());
                AssetSetError::Uninitialized
            }),
            BevyAssetClone::Loaded(change_tracked) => {
                change_tracked.value = value;
                change_tracked.dioxus_changed = true;
                Ok(())
            },
        }
    } 
    pub fn get(&self) -> Option<&T> {
        match self {
            BevyAssetClone::Loaded(value) => Some(value.get()),
            _ => None
        }
    }
    pub fn loaded(value: T) -> Self {
        Self::Loaded(ChangeTracked { value, dioxus_changed: false })
    }
}

/// Hook to request the bevy asset's Assets<T>
pub fn use_bevy_assets<T: Debug + Asset + Clone + Send + Sync + 'static>() -> CrossDomSignal<BevyAssetsValue<T>> {
    use_bevy_value::<T, BevyAssetsHolder<T>, RequestBevyAssets<T>, BevyAssetsValue<T>>()
}

pub fn use_bevy_asset_handle<T: Debug + Asset + Clone + Send + Sync + 'static>(handle: &Handle<T>) -> Result<CrossDomSignal<BevyAssetClone<T>>, AssetGetError> {
    let Ok(asset_server) = use_bevy_assets::<T>().get() else {
        warn!("asset server uninitialized");
        return Err(AssetGetError::Uninitialized)
    };
    let Some(asset) = asset_server.get(handle) else {
        warn!("asset doesn't exist");
        return Err(AssetGetError::DoesntExist)
    };
    Ok(asset.clone())
}