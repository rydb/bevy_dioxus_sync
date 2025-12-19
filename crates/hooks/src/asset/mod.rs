use std::any::TypeId;
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
use dioxus_hooks::Dependency;

use crate::use_bevy_value;

// pub fn use_bevy_asset<T: Asset>(handle: Handle<T>) -> BevyAssetClone<T> {
//     use_bevy
// }

#[derive(Resource, Default)]
pub struct CheckedAssetTypes(HashSet<TypeId>);

#[derive(Clone)]
pub struct RequestBevyAssets<T: Asset>(BevyAssetsClone<T>);

impl<T: PartialEq + Asset + Clone> Command for RequestBevyAssets<T> {
    fn apply(self, world: &mut bevy_ecs::world::World) -> () {
        
        match world.get_resource::<BevyAssetsClone<T>>() {
            Some(n) => self.0.0.pnt_to(n.0.get_ptr().unwrap()),
            None => {
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

pub fn manage_asset_status<T: Clone + Asset + PartialEq>(
    mut assets: ResMut<Assets<T>>,
    assets_clone: ResMut<BevyAssetsClone<T>>,
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
        
        let mut asset = match assets.get_mut(handle) {
            Some(n) => {
                n
            },
            None => todo!("implement behaviour for when asset is removed but handles for it still exist in dioxus."),
        };
        // bevy side asset updates take priority over dioxus side updates, so their update over-rides a dioxus update by moving first if they are both changed at the same time.
        if change_event_asset_ids.contains(&&handle.id()) {
            let _ = signal.set(BevyAssetClone::Loaded(asset.clone())).inspect_err(|err| warn!("{err}"));
            continue;
        } else {
            let Ok(signal_value) = signal.get().inspect_err(|err| warn!("{err}")) else  {
                continue
            };
            match signal_value.as_ref() {
                BevyAssetClone::Loading(_handle) => {
                    let _ = signal.set(BevyAssetClone::Loaded(asset.clone())).inspect_err(|err| warn!("{err}"));
                },
                BevyAssetClone::Loaded(current_value) => {
                    if asset != current_value {
                        warn!("setting new asset value..");
                        asset = &mut current_value.clone();
                        //let _ = signal.set(BevyAssetClone::Loaded(asset.clone())).inspect_err(|err| warn!("{err}"));
                    }
                } ,
            }
        }
    }
} 

pub type BevyAssetServerCloneSignal<T> = CrossDomSignal<HashMap<Handle<T>, CrossDomSignal<BevyAssetClone<T>>>>;

#[derive(Resource, Clone)]
pub struct BevyAssetsClone<T: Asset>(BevyAssetServerCloneSignal<T>);

pub enum BevyAssetClone<T: Asset> {
    Loading(Handle<T>),
    Loaded(T)
}

// /// Hook to request the bevy asset server 
// pub fn use_bevy_asset_server<T: Asset>() -> BevyAssetServerCloneSignal<T> {
//     use_bevy_value::<T>()
// }