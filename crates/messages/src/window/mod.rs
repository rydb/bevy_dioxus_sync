use bevy_asset::prelude::*;
use bevy_dioxus_tracing::debug;
use bevy_ecs::prelude::*;
use bevy_image::prelude::*;
use bevy_math::prelude::*;
use bevy_transform::components::Transform;
use bevy_window::WindowResized;

use bevy_dioxus_render::{DioxusUiQuad, TextureImage, create_ui_texture};

pub(crate) fn handle_window_resize(
    mut resize_events: MessageReader<WindowResized>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut quad_query: Query<&mut Transform, With<DioxusUiQuad>>,
) {
    // Process only the last resize event per frame to prevent texture
    // thrashing from starving the GPU pipeline during live resize.
    let last_event = resize_events.read().last();

    if let Some(resize_event) = last_event {
        let width = resize_event.width as u32;
        let height = resize_event.height as u32;

        debug!("Window resized to: {}x{}", width, height);

        // Create a new texture with the new size.
        let new_image = create_ui_texture(width, height);
        let new_handle = images.add(new_image);

        // Scale the quad to fill the new window dimensions immediately
        // so the old texture stretches to fill rather than leaving gaps.
        if let Ok(mut trans) = quad_query.single_mut() {
            *trans = Transform::from_scale(Vec3::new(width as f32, height as f32, 0.0));
        }

        // Old texture stays in Assets to keep cached texture views valid
        // until the render world sends back the replacement GPU texture.
        commands.insert_resource(TextureImage(new_handle));
    }
}
