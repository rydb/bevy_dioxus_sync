use std::{
    any::{Any, TypeId, type_name}, collections::HashMap, fmt::Display, sync::Arc
};

use bevy_ecs::{prelude::*, schedule::ScheduleLabel, system::ScheduleSystem, world::CommandQueue};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver as CrossBeamReceiver, Sender as CrossBeamSender};
use dioxus_native_dom::DioxusDocument;
use generational_box::GenerationalRef;
use tokio::sync::broadcast::{self, Receiver as TokioReceiver, Sender as TokioSender};
use crossbeam_channel::Sender;

use crate::signals::CrossDomSignal;

pub mod plugins;
pub mod traits;
pub mod signals;
/// Bevy side channel for giving [`T`] to dioxus
// #[derive(Resource)]
// pub struct BevyTxChannel<T>(pub TokioSender<T>);

/// Dioxus side channel for receiving [`T`] from bevy.
#[derive(Resource)]
pub struct BevyRxChannel<T>(pub CrossBeamReceiver<T>);

#[derive(Resource)]
pub struct DioxusCommandQueueRx(pub CrossBeamReceiver<CommandQueue>);

/// An untyped hashmap that resolved typed entries by their type id.
pub type ArcAnytypeMap = HashMap<TypeId, Arc<dyn Any + Send + Sync>>;

pub type BoxAnyTypeMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;

pub type BoxAnySignalTypeMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;

#[derive(Clone, Debug, PartialEq)]
pub enum InfoPacket<T, U, V> {
    Update(InfoUpdate<T, U, V>),
    Request(StatusUpdate),
}

#[derive(Clone, Debug, PartialEq)]
pub enum StatusUpdate {
    /// request a refresh
    RequestRefresh,
}

/// data to/from dioxus
#[derive(Clone, Debug, PartialEq)]
pub struct InfoUpdate<T, U, V> {
    pub update: T,
    pub index: Option<U>,
    pub additional_info: Option<V>,
}
#[derive(Clone)]
pub struct BevyDioxusPacket<T: Send + Sync + Clone + 'static, Index, AdditionalInfo: Clone> {
    pub io: BevyDioxusIO<T, Index, AdditionalInfo>,
    pub signal: CrossDomSignal<Option<T>>,
}

impl<T: Clone + Send + Sync, Index: Clone, AdditionalInfo: Clone> Default for BevyDioxusPacket<T, Index, AdditionalInfo> {
    fn default() -> Self {
        Self { io: Default::default(), signal: Default::default() }
    }
}

/// channels for bevy dioxus interop.
#[derive(Clone)]
pub struct BevyDioxusIO<
    T: Clone,
    Index,
    AdditionalInfo: Clone,
> {
    // pub bevy_tx: TokioSender<InfoPacket<A, Index, AdditionalInfo>>,
    pub bevy_rx: CrossBeamReceiver<InfoPacket<T, Index, AdditionalInfo>>,
    pub dioxus_tx: CrossBeamSender<InfoPacket<T, Index, AdditionalInfo>>,
    // pub dioxus_rx: TokioReceiver<InfoPacket<A, Index, AdditionalInfo>>,
}

impl<T: Clone, Index: Clone, AdditionalInfo: Clone> Default
    for BevyDioxusIO<T, Index, AdditionalInfo>
{
    fn default() -> Self {
        // let (bevy_tx, dioxus_rx) = broadcast::channel(16);
        let (dioxus_tx, bevy_rx) =
            crossbeam_channel::unbounded::<InfoPacket<T, Index, AdditionalInfo>>();

        Self {
            // bevy_tx,
            bevy_rx,
            dioxus_tx,
            // dioxus_rx,
        }
    }
}

/// adds the given system(s), [`T`], to the given system schedule.
pub fn add_systems_through_world<T>(
    world: &mut World,
    schedule: impl ScheduleLabel,
    systems: impl IntoScheduleConfigs<ScheduleSystem, T>,
) {
    let mut schedules = world.get_resource_mut::<Schedules>().unwrap();
    if let Some(schedule) = schedules.get_mut(schedule) {
        schedule.add_systems(systems);
    }
}

/// command queue tx for dioxus -> bevy.
#[derive(Clone)]
pub struct BevyCommandQueueTx(pub CrossBeamSender<CommandQueue>);

/// refresh rate for info sent to dioxus. Lower means more frequent refreshes.
#[derive(Clone)]
pub struct InfoRefershRateMS(pub u64);

impl Default for InfoRefershRateMS {
    fn default() -> Self {
        Self(30)
    }
}

pub(crate) fn read_dioxus_command_queues(world: &mut World) {
    let receiver = world
        .get_resource_mut::<DioxusCommandQueueRx>()
        .unwrap()
        .0
        .clone();
    while let Ok(mut command_queue) = receiver.try_recv() {
        world.commands().append(&mut command_queue);
    }
}

pub struct DioxusDocuments(pub HashMap<Entity, DioxusDocument>);


pub enum BevyFetchBackup {
    /// Return value as unknown as it couldn't be fetched
    Unknown,
    /// Return value for when the value exists in bevy, but dioxus hasn't received it yet.
    Uninitialized,

    ReadError(generational_box::BorrowError)
}

impl From<generational_box::BorrowError> for BevyFetchBackup {
    fn from(value: generational_box::BorrowError) -> Self {
        Self::ReadError(value)
    }
}

impl Default for BevyFetchBackup {
    fn default() -> Self {
        BevyFetchBackup::Unknown
    }
}

impl Display for BevyFetchBackup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            BevyFetchBackup::Unknown => format!("Unable to receive {}", type_name::<Self>()),
            BevyFetchBackup::Uninitialized => "waiting for value from bevy....".to_string(),
            BevyFetchBackup::ReadError(borrow_error) => format!("read error: {}", borrow_error),
        };
        write!(f, "{}", string)
    }
}


// /// bevy value + useful structures needed for bevy/dioxus interop
// pub struct BevyValue<T: Clone + 'static, Index, U> {
//     pub writer: Option<Sender<InfoPacket<T, Index, U>>>,
//     pub reader: Option<TokioReceiver<InfoPacket<T, Index, U>>>,
//     pub value: Result<T, BevyFetchBackup>,
//     pub additional_info: Option<U>,
//     /// index for underlying value, weather this is a number or a untyped handle for an asset
//     pub index: Option<Index>,
// }


/// bevy value + useful structures needed for bevy/dioxus interop
pub struct BevyValue<T: Clone + 'static, Index, U> {
    pub value: CrossDomSignal<T>,
    pub writer: Sender<InfoPacket<T, Index, U>>,
    pub additional_info: Option<U>,
    pub index: Option<Index>,
}

// impl<T: Clone + 'static, Index, U> PartialEq for BevyValue<T, Index, U> {
//     fn eq(&self, other: &Self) -> bool {
//         self.value == other.value
//     }
// }

// impl<T: Clone, Index, U> Readable for BevyValue<T, Index, U> {
//     type Target = T;

//     type Storage = SyncStorage;

//     fn try_read_unchecked(
//         &self,
//     ) -> std::result::Result<ReadableRef<'static, Self>, generational_box::BorrowError>
//     where
//         Self::Target: 'static {
//         todo!()
//     }

//     fn try_peek_unchecked(
//         &self,
//     ) -> std::result::Result<ReadableRef<'static, Self>, generational_box::BorrowError>
//     where
//         Self::Target: 'static {
//         todo!()
//     }

//     fn subscribers(&self) -> Subscribers
//     where
//         Self::Target: 'static {
//         todo!()
//     }
// }
// impl<T: Clone + Readable +  'static, Index: Clone, U: Clone> BevyValue<T, Index, U> {
//     pub fn set_value(&mut self, value: T) {
//         // if let Some(send_channel) = &self.writer {
//             let packet = InfoUpdate {
//                 update: value.clone(),
//                 index: self.index.clone(),
//                 additional_info: self.additional_info.clone(),
//             };
//             let _send_result = self.writer
//                 .send(InfoPacket::Update(packet))
//                 .inspect_err(|err| warn!("could not update bevy value signal due to {:#}", err));
//             // if send_result.is_ok() {
//             //     self.value = Ok(value)
//             // }
        
//         // else {
//         //     warn!("no send channel for {:#}, skipping", type_name::<T>());
//         //     return;
//         // }
//     }

//     // pub fn read_value(&self) -> Result<ReadSignal<T>, BevyFetchBackup> {
//     //     match self.value {
//     //         Some(n) => {
//     //             n
//     //             // match n.read() {
//     //             //     Ok(n) => Ok(n),
//     //             //     Err(err) => Err(BevyFetchBackup::ReadError(err)),
//     //             // }
//     //         },
//     //         None => todo!(),
//     //     }
//     //     // self.value.map(|n | n.read)
//     // }
// }

impl<T, Index, U> Display for BevyValue<T, Index, U>
where
    T: Display + Clone,
    Option<T>: Display
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let binding = self.value.try_read();
        let maybe_value = self.value.get();
        // match maybe_value {
        //     Ok(n) => {
        //         match n.as_ref() {
        //             Some(n) => write!(f, "{}", n),
        //             None => write!(f, "waiting for value from bevy..."),
        //         }
        //     },
        //     Err(err) => write!(f, "borrow error: {err}"),
        // }
        match maybe_value {
            Ok(n) => {
                write!(f, "{}", n)
                // match n.as_ref() {
                    // Some(n) => write!(f, "{}", n),
                    // None => write!(f, "waiting for value from bevy..."),
                // }
            },
            Err(err) => write!(f, "{}", err),
        }
    }
}



// pub struct BevyResourceSignals(HashMap<Entity, HashMap<TypeId, BoxAnyTypeMap>>)