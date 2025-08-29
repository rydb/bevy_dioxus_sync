// //! hook synchronization for components that are newtypes around asset handles to syncronize those underlying assets.

// use std::collections::HashSet;
// use std::default;
// use std::{marker::PhantomData, ops::Deref};

// use async_std::task::sleep;
// use bevy_app::Update;
// use bevy_asset::*;
// use bevy_ecs::prelude::*;
// use bevy_ecs::query::QueryFilter;
// use bevy_ecs::world::CommandQueue;
// use bevy_utils::default;
// use dioxus::hooks::{use_context, use_context_provider, use_future};
// use dioxus::signals::{ReadableExt, Signal, SignalSubscriberDrop, SyncSignal, UnsyncStorage, WriteLock};
// use crate::dioxus_in_bevy_plugin::DioxusProps;
// use crate::queries_sync::one_component_kind::{fallback_update_removed_components, AddRemoveQueues, ComponentsErased};
// use crate::queries_sync::BevyDioxusIO;
// use crate::traits::{ErasedSubGenericAssetsMap, ErasedSubGenericComponentsMap};
// use crate::*;

// pub enum AssetLoadState<T> {
//     Loaded(T),
//     Loading,
//     Failed(String)
// }

// pub struct RequestBevyAsset<T, U: Asset> {
//     io: BevyDioxusIO<AssetIOQueue<U>, HashMap<Handle<U>, U>>,
//     _phantom: PhantomData<T>,
// }

// impl<T, U> RequestBevyAsset<T, U> 
//     where
//         T: Deref<Target = Handle<U>> + Component,
//         U: Asset + Clone
// {
//         pub fn new<'a>() -> Self {
//         let (bevy_tx, dioxus_rx) = crossbeam_channel::unbounded::<AssetIOQueue<U>>();
//         let (dioxus_tx, bevy_rx) = crossbeam_channel::unbounded::<HashMap<Handle<U>, U>>();

//         Self {
//             io: BevyDioxusIO {
//                 dioxus_tx,
//                 dioxus_rx,
//                 bevy_tx,
//                 bevy_rx,
//             },
//             _phantom: PhantomData,
//         }
//     }
// }

// impl<T, U> Command for RequestBevyAsset<T, U>
//     where
//         T: Deref<Target = Handle<U>> + Component,
//         U: Asset + Clone
// {
//     fn apply(self, world: &mut World) -> () {
//         world.insert_resource(BevyTxChannel(self.io.bevy_tx.clone()));
//         world.insert_resource(BevyRxChannel(self.io.bevy_rx));

//         let component_hook = world
//         .register_component_hooks::<T>()
//         .try_on_remove(|mut world, hook| {
//             let bevy_tx = world.get_resource_mut::<BevyTxChannel<AssetIOQueue<U>>>().unwrap().0.clone();

//             let value = match world.entity(hook.entity).get::<T>() {
//                 Some(val) => val,
//                 None => {
//                     warn!(
//                         "could not get {:#} on {:#}", type_name::<T>(), hook.entity
//                     );
//                     return;
//                 }
//             };

//             let handle = (**value).clone();

//             let mut remove = HashSet::new();
            


//             remove.insert(handle);

//             let new_requests = AddRemoveQueues {
//                 add: Default::default(),
//                 remove,
//             };

//             let _ = bevy_tx.send(new_requests).inspect_err(|err| warn!("could not send remove request for {:#} for {:#} due to {:#}", hook.entity, type_name::<T>(), err));
//         });

//         if component_hook.is_none() {
//             warn!("could not add .on_remove hook for {:#} because it already has one, using manual system check instead", type_name::<T>());
//             add_systems_through_world(world, Update, fallback_update_removed_components::<T>);
//         }
//         // populate initial component map set
//         {
//             let mut add = HashMap::default();
//             let mut components = world.query::<(Entity, &T)>();

            
//             for (e, handle) in components.iter(world) {
//                 add.insert((**handle).clone(), AssetLoadState::Loading);
//             }
//             let _ = self.io.bevy_tx.send(AddRemoveQueues{
//                 add,
//                 remove: Default::default()
//             }).inspect_err(|err| warn!("Could not send initial component map due to: {:#}", err));
//         }
//         // add_systems_through_world(world, Update, try_load_initial_asset::<T, U>);
//         add_systems_through_world(world, Update, send_updated_asset::<T, U>);
//         add_systems_through_world(world, Update, receive_updated_asset::<T, U>);


//     }
// }

// /// Component that asset T has been initialized within dioxus and is being tracked
// #[derive(Component)]
// pub struct DioxusTrackedAsset<T, U>
//     where
//         T: Deref<Target = Handle<U>> + Component,
//         U: Asset + Clone
// {
//     phantom_a: PhantomData<T>,
//     phantom_b: PhantomData<U>,
// }

// impl<T, U> Default for DioxusTrackedAsset<T, U> 
//     where
//         T: Deref<Target = Handle<U>> + Component,
//         U: Asset + Clone
// {
//     fn default() -> Self {
//         Self { phantom_a: Default::default(), phantom_b: Default::default() }
//     }
// }

// fn get_asset_status<T, U, V: QueryFilter>(
//     asset_server: Res<AssetServer>,
//     assets: Res<Assets<U>>,
//     dioxus_asset_requests: Query<(Entity, &T), V>,
// ) -> HashMap<Handle<U>, AssetLoadState<U>> 
//     where
//         T: Deref<Target = Handle<U>> + Component,
//         U: Asset + Clone
// {
//     let mut map = HashMap::new();
//     for (e, handle) in dioxus_asset_requests {
//         let id = handle.id();
//         let handle = (**handle).clone();
//         if let Some(asset) = assets.get(id) {
//             map.insert(handle, AssetLoadState::Loaded(asset.clone()));
//         } else {
//             let load_status = asset_server.load_state(id);

//             match load_status {

//                 LoadState::Failed(asset_load_error) => {
//                     map.insert(handle, AssetLoadState::Failed(asset_load_error.to_string()));

//                 },
//                 LoadState::Loading => {
//                     map.insert(handle, AssetLoadState::Loading);
//                 }
//                 _ => continue
//             }
//         }
//     }
//     map
// }

// // fn try_load_initial_asset<T, U>(
// //     asset_server: Res<AssetServer>,
// //     assets: Res<Assets<U>>,
// //     dioxus_asset_requests: Query<(Entity, &T), Without<DioxusTrackedAsset<T, U>>>,
// //     bevy_tx: ResMut<BevyTxChannel<AssetIOQueue<U>>>,
// //     mut commands: Commands
// // ) 
// //     where
// //         T: Deref<Target = Handle<U>> + Component,
// //         U: Asset + Clone
// // {
// //     let asset_statuses = get_asset_status(asset_server, assets, dioxus_asset_requests);
// //     for (e, statuses) in &asset_statuses {
// //         match statuses {
// //             AssetLoadState::Loaded(_) => {
// //                 commands.entity(*e).insert(DioxusTrackedAsset::<T, U>::default());
// //             },
// //             AssetLoadState::Loading => {
// //                 // re-run this system until the asset is done loading.
// //             },
// //             AssetLoadState::Failed(_) => {
// //                 commands.entity(*e).insert(DioxusTrackedAsset::<T, U>::default());
// //             },
// //         }
// //     }

// //     let _ = bevy_tx.0.send(AddRemoveQueues {
// //         add: asset_statuses,
// //         ..default()
// //     }).inspect_err(|err| warn!("could not send loaded initial asset for {:#} due to to: {:#}", type_name::<T>(), err));
// // }

// fn send_updated_asset<T, U>(
//     asset_server: Res<AssetServer>,
//     assets: Res<Assets<U>>,
//     //dioxus_asset_requests: Query<(Entity, &T), (With<DioxusTrackedAsset<T, U>>, Changed<T>)>,
//     dioxus_asset_requests: Query<(Entity, &T), Changed<T>>,

//     bevy_tx: ResMut<BevyTxChannel<AssetIOQueue<U>>>,
//     //mut commands: Commands
// )
//     where
//         T: Deref<Target = Handle<U>> + Component,
//         U: Asset + Clone
// {
//     let asset_statuses = get_asset_status(asset_server, assets, dioxus_asset_requests);

//     let _ = bevy_tx.0.send(AddRemoveQueues {
//         add: asset_statuses,
//         ..default()
//     });

// }

// fn receive_updated_asset<T, U>(
//     bevy_rx: ResMut<BevyRxChannel<HashMap<Handle<U>, U>>>,
//     bevy_tx: ResMut<BevyTxChannel<AssetIOQueue<U>>>,
//     // handles: Query<(Entity, &T)>,
//     mut assets: ResMut<Assets<U>>,
// )
//     where
//         T: Deref<Target = Handle<U>> + Component,
//         U: Asset + Clone
// {
//     let mut remove_queue = HashSet::new();
//     while let Ok(map) = bevy_rx.0.try_recv() {
//         for (handle, new_asset) in map {
//             assets.insert(handle.id(), new_asset)
//             // if let Ok((e, asset)) = handles.get(e) {
//             //     assets.insert(asset.id(), new_asset);
//             // }  else {
//             //     remove_queue.insert(e);
//             // };
//         }
//     }
//     if remove_queue.is_empty() == false {
//         let _ = bevy_tx.0.send(AddRemoveQueues {
//             remove: remove_queue,
//             ..default()
//         }).inspect_err(|err| warn!("could not tell dioxus to remove no-longer valid for assets {:#} due to {:#}", type_name::<T>(), err));
//     }
// }


// #[derive(TransparentWrapper, Default)]
// #[repr(transparent)]
// pub struct AssetErased(BoxAnyTypeMap);


// #[derive(Clone, PartialEq, Eq, Hash)]
// pub struct BevyAssetHandle<T: Asset + Clone>(pub Option<Handle<T>>);

// impl<T: Asset + Clone> BevyAssetHandle<T> {
//     pub fn set_asset(&self, value: T, assets: &mut BevyAssetsProcessed<T>) {
//         let Some(handle) = &self.0 else {
//             warn!("could not write asset because its handle isn't initialized yet.");
//             return;
//         };
//         let mut add = HashMap::new();
//         add.insert(handle.clone(), AssetLoadState::Loaded(value));
//         let _ = assets.write.send(AssetIOQueue { add, ..default() }).inspect_err(|err| warn!("could not write asset {:#}", err));
//     }
//     pub fn read_asset<'a>(&self, asset_server: &'a mut BevyAssetsProcessed<T>) -> Option<&'a T> {
//         let Some(handle) = &self.0  else{
//           return None;
//         };
//         asset_server.read_assets.get(handle)

//     }
// }

// pub type AssetIOQueue<T> = AddRemoveQueues<Handle<T>, AssetLoadState<T>>;


// pub struct BevyAssetsProcessed<T: Asset + Clone> {
//     pub read_assets: HashMap<Handle<T>, T>,
//     pub write: Sender<AssetIOQueue<T>>,
// }




// pub struct BevyAssetsUnprocessed<T: Asset + Clone> {
//     read: Receiver<AssetIOQueue<T>>,
// }


// fn request_asset_processor<U: Asset + Clone>() -> SyncSignal<BevyAssetsProcessed<U>> {

// }

// fn request_asset_channels<T, U>(
//     props: DioxusProps,
//     mut signal_registry: WriteLock<
//         '_,
//         AssetHandlesErased,
//         UnsyncStorage,
//         SignalSubscriberDrop<AssetHandlesErased, UnsyncStorage>,
//     >,
// ) -> SyncSignal<BevyAssetHandle<U>> 
//     where
//         T: Deref<Target = Handle<U>> + Component,
//         U: Asset + Clone

// {
//     let mut commands = CommandQueue::default();

//     let command = RequestBevyAsset::<T, U>::new();

//     let Some(processed_assets) = use_context::<BevyAssetsProcessedSignal>().0.read().get::<U>();
//     use_future(move || {
//         // let value = props.clone();
//         async move {
//             // let mut signal = signal.clone();
//             loop {
//                 sleep(std::time::Duration::from_millis(1000)).await;

//                 let mut resource = signal.write();
//                 warn!("attempting to receive resource");
//                 while let Ok(value) = resource.resource_incoming.try_recv() {
//                     warn!("received value: {:#?}", value);
//                     resource.resource_read = Some(value)
//                 }
//             }
//         }
//     });


//     commands.push(command);

//     let new_signal = SyncSignal::new_maybe_sync(BevyAssetHandle(None));

//     signal_registry.insert(new_signal.clone());
//     let _ =props.command_queues_tx.send(commands).inspect_err(|err| warn!("could not request component channel for {:#}: {:#}", type_name::<T>(), err));

//     return new_signal;
// }

// fn receive_asset_updates<U> () {
//     let props = use_context::<DioxusProps>();

// }



// fn process_asset_updates() {
//     let unprocessed_assets = use_context_provider::<BevyAssetsUnprocessedSignal>(|| BevyAssetsUnprocessedSignal::default());
// }

// pub fn use_bevy_asset<T, U>() 
//     where
//         T: Deref<Target = Handle<U>> + Component,
//         U: Asset + Clone
// {
//     let props = use_context::<DioxusProps>();

//     let mut asset_signals = use_context::<BevyAssetHandlesSignals>();

// }




// #[derive(TransparentWrapper, Default)]
// #[repr(transparent)]
// pub struct AssetHandlesErased(BoxAnyTypeMap);

// impl ErasedSubGenericAssetsMap for AssetHandlesErased {
//     type Generic<T: Clone + Asset + Send + Sync + 'static> = SyncSignal<BevyAssetHandle<T>>;
// }


// #[derive(TransparentWrapper, Default)]
// #[repr(transparent)]
// pub struct AssetsProcessedErased(BoxAnyTypeMap);

// impl ErasedSubGenericAssetsMap for AssetsProcessedErased {
//     type Generic<T: Clone + Asset + Send + Sync + 'static> = SyncSignal<BevyAssetsProcessed<T>>;
// }

// #[derive(TransparentWrapper, Default)]
// #[repr(transparent)]
// pub struct AssetsUnprocessedErased(BoxAnyTypeMap);

// impl ErasedSubGenericAssetsMap for AssetsUnprocessedErased {
//     type Generic<T: Clone + Asset + Send + Sync + 'static> = SyncSignal<BevyAssetsProcessed<T>>;
// }

// #[derive(Clone, Default)]
// pub struct BevyAssetHandlesSignals(pub Signal<AssetHandlesErased>);

// #[derive(Default, Clone)]
// pub struct BevyAssetsProcessedSignal(pub Signal<AssetsProcessedErased>);

// #[derive(Default, Clone)]
// pub struct BevyAssetsUnprocessedSignal(pub Signal<AssetsUnprocessedErased>);

// // impl ErasedSubGenericAssetsMap for ComponentsErased {
// //     type Generic<T: Clone + Asset + Send + Sync + 'static> = SyncSignal<BevyQueryComponents<T>>;
// // }Be