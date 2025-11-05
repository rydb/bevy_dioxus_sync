use bevy_app::Update;
use bevy_dioxus_interop::{
    BevyDioxusIO, BevyRxChannel, BevyTxChannel, InfoPacket, InfoUpdate, StatusUpdate,
    add_systems_through_world,
};
use bevy_ecs::{component::Mutable, prelude::*};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use std::{any::TypeId, marker::PhantomData};

type ComponentValue<T> = T;
type ComponentIndex = TypeId;
type ComponentAdditionalInfo = ();

/// info sent to/from bevy on component data.
type ComponentInfoPacket<T> =
    InfoPacket<ComponentValue<T>, ComponentIndex, ComponentAdditionalInfo>;

/// Command to register dioxus bevy interop for a given resource.
#[derive(TransparentWrapper)]
#[repr(transparent)]
#[transparent(BevyDioxusIO<ComponentValue<T>, ComponentIndex, ComponentAdditionalInfo>)]
pub(crate) struct RequestBevyComponentSingleton<
    T: Component<Mutability = Mutable> + Clone,
    U: Component + Clone,
> {
    pub(crate) channels: BevyDioxusIO<ComponentValue<T>, ComponentIndex, ComponentAdditionalInfo>,
    singleton_marker: PhantomData<U>,
}

impl<T, U> Default for RequestBevyComponentSingleton<T, U>
where
    T: Component<Mutability = Mutable> + Clone,
    U: Component + Clone,
{
    fn default() -> Self {
        Self {
            channels: BevyDioxusIO::default(),
            singleton_marker: PhantomData,
        }
    }
}

impl<T, U> Command for RequestBevyComponentSingleton<T, U>
where
    T: Component<Mutability = Mutable> + Clone,
    U: Component + Clone,
{
    fn apply(self, world: &mut World) -> () {
        world.insert_resource(BevyTxChannel(self.channels.bevy_tx));
        world.insert_resource(BevyRxChannel(self.channels.bevy_rx));

        add_systems_through_world(world, Update, send_component_singleton::<T, U>);
        add_systems_through_world(world, Update, receive_component_update::<T, U>);
    }
}

fn send_component_singleton<T, U>(
    singleton: Query<(&T, &U), Changed<T>>,
    bevy_tx: ResMut<BevyTxChannel<ComponentInfoPacket<T>>>,
) where
    T: Component + Clone,
    U: Component,
{
    let Ok((value, _)) = singleton.single() else {
        return;
    };
    let packet = InfoUpdate {
        update: value.clone(),
        index: Some(TypeId::of::<T>()),
        additional_info: None,
    };
    let _ = bevy_tx
        .0
        .send(InfoPacket::Update(packet))
        .inspect_err(|err| warn!("could not send component: {:#}", err));
}

fn receive_component_update<T, U>(
    mut singleton: Query<&mut T, With<U>>,
    bevy_rx: ResMut<BevyRxChannel<ComponentInfoPacket<T>>>,
    bevy_tx: ResMut<BevyTxChannel<ComponentInfoPacket<T>>>,
) where
    T: Component<Mutability = Mutable> + Clone,
    U: Component,
{
    let Ok(mut singleton) = singleton.single_mut() else {
        return;
    };
    let Ok(packet) = bevy_rx.0.try_recv() else {
        return;
    };
    match packet {
        InfoPacket::Update(info_update) => *singleton = info_update.update,
        InfoPacket::Request(status_update) => match status_update {
            StatusUpdate::RequestRefresh => {
                let packet = InfoUpdate {
                    update: singleton.clone(),
                    index: Some(ComponentIndex::of::<T>()),
                    additional_info: Some(()),
                };
                let _ = bevy_tx
                    .0
                    .send(InfoPacket::Update(packet))
                    .inspect_err(|err| warn!("could not send component: {:#}", err));
            }
        },
    }
}
