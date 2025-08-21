use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_image::prelude::*;
use bevy_log::debug;
use bevy_math::prelude::*;
use bevy_sprite::{ColorMaterial, MeshMaterial2d};
use bevy_transform::components::Transform;
use bevy_window::WindowResized;
use blitz_traits::shell::Viewport;
use dioxus_native::DioxusDocument;

use crate::render::{COLOR_SCHEME, DioxusUiQuad, SCALE_FACTOR, TextureImage, create_ui_texture};

pub(crate) fn handle_window_resize(
    mut dioxus_doc: NonSendMut<DioxusDocument>,
    mut resize_events: EventReader<WindowResized>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    texture_image: Option<Res<TextureImage>>,
    mut query: Query<(&mut Transform, &mut MeshMaterial2d<ColorMaterial>), With<DioxusUiQuad>>,
) {
    for resize_event in resize_events.read() {
        let width = resize_event.width as u32;
        let height = resize_event.height as u32;

        debug!("Window resized to: {}x{}", width, height);

        // Update the dioxus viewport
        dioxus_doc.set_viewport(Viewport::new(width, height, SCALE_FACTOR, COLOR_SCHEME));
        dioxus_doc.resolve();

        // Create a new texture with the new size
        let new_image = create_ui_texture(width, height);
        let new_handle = images.add(new_image);

        // Update the quad mesh to match the new size
        if let Ok((mut trans, mut mat)) = query.single_mut() {
            *trans = Transform::from_scale(Vec3::new(width as f32, height as f32, 0.0));
            materials.get_mut(&mut mat.0).unwrap().texture = Some(new_handle.clone());
        }

        // Remove the old texture
        if let Some(texture_image) = texture_image.as_ref() {
            images.remove(&texture_image.0);
        }

        // Insert the new texture resource
        commands.insert_resource(TextureImage(new_handle));
    }
}
