use async_std::task::sleep;
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, world::CommandQueue};
use crossbeam_channel::{Receiver, Sender};
use dioxus::{core::use_hook, hooks::{use_context, use_future}, signals::{Signal, SignalSubscriberDrop, SyncSignal, UnsyncStorage, WritableExt, WriteLock}};
use std::fmt::{Debug, Display};

use crate::{dioxus_in_bevy_plugin::DioxusProps, traits::ErasedSubGenericResourcecMap, *};


pub mod command;
pub mod hook;



