use std::any::TypeId;

use bevy_app::prelude::*;
use bevy_dioxus_interop::{
    BevyDioxusIO, BevyRxChannel, BevyTxChannel, InfoPacket, InfoUpdate, StatusUpdate, add_systems_through_world
};
use bevy_ecs::prelude::*;
use bevy_log::warn;
use bytemuck::TransparentWrapper;

#[derive(TransparentWrapper)]
#[repr(transparent)]
pub struct RequestBevyResource<T: Resource + Clone>(
    pub(crate) BevyDioxusIO<ResourceValue<T>, ResourceInfoIndex, ResourceAdditionalInfo>,
);

// lay out types like this to prevent de-sync between systems and backend logistics updates.
type ResourceInfoIndex = TypeId;
type ResourceValue<T> = T;
type ResourceAdditionalInfo = ();
type ResourceInfoPacket<T> = InfoPacket<ResourceValue<T>, ResourceInfoIndex, ResourceAdditionalInfo>;

impl<T: Resource + Clone> Default for RequestBevyResource<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: Resource + Clone> Command for RequestBevyResource<T> {
    fn apply(self, world: &mut World) -> () {
        // let x = Channels::<Self>::default();
        world.insert_resource(BevyTxChannel(self.0.bevy_tx));
        world.insert_resource(BevyRxChannel(self.0.bevy_rx));

        add_systems_through_world(
            world,
            Update,
            send_resource_update::<T>.run_if(resource_changed::<T>),
        );
        add_systems_through_world(world, Update, receive_resource_update::<T>);
    }
}

fn send_resource_update<T: Resource + Clone>(
    resource: Res<T>,
    bevy_tx: ResMut<BevyTxChannel<ResourceInfoPacket<T>>>,
) {
    let packet = InfoUpdate {
        update: resource.clone(),
        index: Some(TypeId::of::<T>()),
        additional_info: Some(()),
    };
    let _ = bevy_tx
        .0
        .send(InfoPacket::Update(packet))
        .inspect_err(|err| warn!("could not send resource: {:#}", err));
}

fn receive_resource_update<T: Resource + Clone>(
    mut resource: ResMut<T>,
    bevy_rx: ResMut<BevyRxChannel<ResourceInfoPacket<T>>>,
    bevy_tx: ResMut<BevyTxChannel<ResourceInfoPacket<T>>>,

) {
    while let Ok(packet) = bevy_rx.0.try_recv()
    .inspect_err(|err| match err {
        crossbeam_channel::TryRecvError::Empty => {},
        crossbeam_channel::TryRecvError::Disconnected => warn!("could not receive as channel is disconnected"),
    }){
        match packet {
            InfoPacket::Update(info_update) => {
                *resource = info_update.update;
            },
            InfoPacket::Request(status_update) => {
                match status_update {
                    StatusUpdate::RequestRefresh => {
                        send_resource_update(resource.into(), bevy_tx)
                    },
                }
            },
        }
        return;
    };
}
