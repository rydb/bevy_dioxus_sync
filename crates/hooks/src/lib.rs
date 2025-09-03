use bytemuck::TransparentWrapper;
use crossbeam_channel::{Receiver, Sender};
use bevy_ecs::{prelude::*, schedule::ScheduleLabel, system::ScheduleSystem};

// pub mod asset_handle;
pub mod asset_single;
pub mod component_single;
pub mod resource;

pub mod traits;
