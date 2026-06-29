use bevy_asset::prelude::*;
use bevy_dioxus_tracing::debug;
use bevy_ecs::prelude::*;
use bevy_image::prelude::*;
use bevy_math::prelude::*;
use bevy_transform::components::Transform;
use bevy_window::{Window, WindowResized};

use bevy_dioxus_render::{DioxusUiQuad, create_ui_texture};

pub(crate) fn handle_window_resize(
    mut resize_events: MessageReader<WindowResized>,
    mut images: ResMut<Assets<Image>>,
    mut quad_query: Query<(&mut Transform, &mut DioxusUiQuad), With<Window>>,
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
        for (mut trans, mut quad) in quad_query.iter_mut() {
            *trans = Transform::from_scale(Vec3::new(width as f32, height as f32, 0.0));
            quad.handle = Some(new_handle.clone());
        }

    }
}
