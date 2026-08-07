pub mod bevy_scene_plugin;

use std::fmt::Display;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;

#[derive(Component, Clone)]
pub struct DynamicCube;

#[derive(Resource, Clone, Debug, Deref, DerefMut, PartialEq)]
pub struct CubeTranslationSpeed(pub f32);

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct FPS(pub f32);

impl Display for FPS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}", self.0)
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

#[derive(Resource, Clone, Debug, Deref, DerefMut, PartialEq)]
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

#[derive(Resource, Clone, Debug, Deref, DerefMut, PartialEq)]
pub struct SignDistance(pub f32);

impl Display for SignDistance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for SignDistance {
    fn default() -> Self {
        Self(-2.0)
    }
}
