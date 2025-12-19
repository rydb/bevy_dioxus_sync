use bevy_dioxus_interop::{
    BevyCommandQueueTx,
    signals::CrossDomSignal,
};
use bevy_ecs::{
    system::Command,
    world::CommandQueue,
};
use bevy_log::warn;
use bytemuck::TransparentWrapper;
use dioxus_core::{provide_context, try_consume_context};
use dioxus_hooks::use_context;

pub mod query;
pub mod resource;
pub mod asset;
// pub mod component;

// /// hook that handles the logistics of getting a value to and from bevy.
// pub fn use_bevy_value<T, V, W, X>(index: V::Index) -> CrossDomSignal<T>
// where
//     T: Send + Sync + Clone + 'static,
//     V: TransparentWrapper<BoxGenericDomTypeMap<V::Index>> + CrossDomSignalErasedMap + 'static,
//     W: Clone + Command + Default + TransparentWrapper<CrossDomSignal<T>>,
//     X: Clone + TransparentWrapper<Signal<V>> + 'static,
// {
//     let command_queue_tx = use_context::<BevyCommandQueueTx>();

//     let instance = use_context::<X>();

//     let mut binding = TransparentWrapper::peel(instance);
//     let mut signals = binding.write();
//     let signal = match signals.get_typed::<T>(&index) {
//         Some(signal) => signal.clone(),
//         None => {
//             let signal = request_bevy_signal::<T, V, W>(command_queue_tx);
//             signals.insert_signal(signal.clone(), index);
//             signal
//         }
//     };
//     signal
// }

pub fn use_bevy_value<BevyValueType, BevyValueHolder, RequestCommand, SignalType>()
-> CrossDomSignal<SignalType>
where
    BevyValueType: 'static,
    BevyValueHolder: Clone + TransparentWrapper<CrossDomSignal<SignalType>> + 'static,
    RequestCommand: TransparentWrapper<BevyValueHolder> + Command,
{
    let command_queue_tx = use_context::<BevyCommandQueueTx>();

    let signal = match try_consume_context::<BevyValueHolder>() {
        Some(context) => context,
        None => {
            let new_signal =
                request_bevy_signal::<BevyValueType, RequestCommand, BevyValueHolder, SignalType>(
                    command_queue_tx,
                );
            provide_context(BevyValueHolder::wrap(new_signal))
        }
    };

    let signal = TransparentWrapper::peel(signal);
    signal
}

fn request_bevy_signal<BevyValue, RequestCommand, BevyValueHolder, SignalType>(
    command_queue_tx: BevyCommandQueueTx,
) -> CrossDomSignal<SignalType>
where
    BevyValueHolder: Clone + TransparentWrapper<CrossDomSignal<SignalType>> + 'static,
    RequestCommand: TransparentWrapper<BevyValueHolder> + Command,
{
    let new_signal: CrossDomSignal<SignalType> = CrossDomSignal::new_uninitialized();

    let bevy_holder_type = BevyValueHolder::wrap(new_signal.clone());
    let mut commands = CommandQueue::default();

    let command = RequestCommand::wrap(bevy_holder_type);

    commands.push(command);

    let _ = command_queue_tx
        .0
        .send(commands)
        .inspect_err(|err| warn!("{err}"));

    return new_signal;
}

// fn request_bevy_signal<T, U, V>(command_queue_tx: BevyCommandQueueTx) -> CrossDomSignal<T>
// where
//     T: 'static + Send + Sync + Clone,
//     U: TransparentWrapper<BoxGenericDomTypeMap<U::Index>> + CrossDomSignalErasedMap,
//     V: Clone + Command + Default + TransparentWrapper<CrossDomSignal<T>>,
// {
//     let new_signal = CrossDomSignal::new_uninitialized();

//     let mut commands = CommandQueue::default();

//     let command = V::wrap(new_signal.clone());
//     commands.push(command);

//     let _ = command_queue_tx
//         .0
//         .send(commands)
//         .inspect_err(|err| warn!("{:#}", err));
//     return new_signal;
// }
