pub mod bevy_scene_plugin;

use std::fmt::Display;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;

#[derive(Component, Clone)]
pub struct DynamicCube;

#[derive(Resource, Clone, Debug, Deref, DerefMut)]
pub struct CubeTranslationSpeed(pub f32);

#[derive(Resource, Debug, Clone)]
pub struct FPS(pub f32);

impl Display for FPS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for CubeTranslationSpeed {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Display for CubeTranslationSpeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Resource, Clone, Debug, Deref, DerefMut)]
pub struct CubeRotationSpeed(pub f32);

impl Display for CubeRotationSpeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for CubeRotationSpeed {
    fn default() -> Self {
        Self(2.0)
    }
}
