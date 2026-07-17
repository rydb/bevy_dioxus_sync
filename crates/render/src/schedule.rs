use std::time::Duration;

use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
use bevy_time::{Time, Virtual};

/// Schedule for dioxus rendering systems

#[derive(ScheduleLabel, Hash, Debug, Eq, PartialEq, Clone)]
pub struct DioxusRenderSchedule;

#[derive(Resource)]
pub struct DioxusRenderScheduleTimestep {
    timestep: Duration,
}

impl DioxusRenderScheduleTimestep {
    pub fn from_fps(fps: u32) -> Self {
        Self {
            timestep: Duration::from_secs_f64(1.0 / fps as f64),
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct DioxusRenderScheduleAccumulator {
    /// The interval between successive runs of the Dioxus render schedules.
    pub accumulated: Duration,
}

pub(crate) struct DioxusRenderMain;

impl DioxusRenderMain {
    pub fn run_dioxus_render_main(world: &mut World) {
        let delta = world.resource::<Time<Virtual>>().delta();
        let timestep = world
            .resource_mut::<DioxusRenderScheduleTimestep>()
            .timestep;

        {
            // set current time
            world
                .resource_mut::<DioxusRenderScheduleAccumulator>()
                .accumulated += delta;
        }

        loop {
            let mut schedule_time = world.resource_mut::<DioxusRenderScheduleAccumulator>();

            let should_run = {
                let acc = schedule_time.accumulated;
                acc >= timestep
            };

            if !should_run {
                break;
            }

            schedule_time.accumulated -= timestep;

            let _ = world.try_run_schedule(DioxusRenderSchedule);
        }
    }
}
