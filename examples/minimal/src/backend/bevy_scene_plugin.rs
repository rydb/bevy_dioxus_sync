use bevy::input::mouse::{MouseButton, MouseMotion};
use bevy::prelude::*;
use bevy_dioxus_render::DioxusUiResolution;
use bevy_dioxus_render::panels::DioxusPanels;
use crate::backend::*;
use crate::frontend::sign_ui::sign_ui;
#[derive(Component)]
pub struct OrbitCamera {
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub sensitivity: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            distance: 3.0,
            yaw: 0.0,
            pitch: 0.0,
            sensitivity: 0.01,
        }
    }
}

#[derive(Component)]
pub struct Signpost;

pub struct BevyScenePlugin;

impl Plugin for BevyScenePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CubeTranslationSpeed::default());
        app.insert_resource(FPS(0.0));
        app.insert_resource(CubeRotationSpeed::default());
        app.insert_resource(SignDistance::default());
        app.add_systems(Startup, (setup_scene, setup_sign).chain());
        app.add_systems(Update, (sync_with_ui, animate, orbit_camera_system));
    }
}

fn setup_sign(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Platform
    commands.spawn((
        // Mesh3d(meshes.add(Cuboid::new(2.0, 0.2, 1.0))),
        Mesh3d(meshes.add(Plane3d::new(Vec3::new(0.0, 1.0, 0.0), Vec2::new(2.0, 2.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::Srgba(bevy::color::Srgba::new(0.3, 0.3, 0.35, 1.0)),
            ..default()
        })),
        Transform::from_xyz(3.0, -0.9, -1.0),
    ));

    // Sign
    commands.spawn((
        Signpost,
        Transform::from_xyz(1.0, -0.5, 2.0),
    )).with_children(|parent| {
        // // Stem
        // parent.spawn((
        //     Mesh3d(meshes.add(Cuboid::new(0.15, 1.2, 0.15))),
        //     MeshMaterial3d(materials.add(StandardMaterial {
        //         base_color: Color::Srgba(bevy::color::Srgba::new(0.4, 0.35, 0.3, 1.0)),
        //         ..default()
        //     })),
        //     Transform::from_xyz(3.0, -0.2, -1.0),
        // ));
        // // Body
        // parent.spawn((
        //     Mesh3d(meshes.add(Cuboid::new(1.4, 0.5, 0.08))),
        //     MeshMaterial3d(materials.add(StandardMaterial {
        //         base_color: Color::Srgba(bevy::color::Srgba::new(0.25, 0.25, 0.3, 1.0)),
        //         ..default()
        //     })),
        //     Transform::from_xyz(3.0, 0.45, -1.0),
        // ));
        // Front
        parent.spawn((
            Mesh3d(meshes.add(Rectangle::new(1.3, 0.45))),
            Transform::from_xyz(0.0, 0.45, -0.96),
            DioxusPanels::new(vec![sign_ui]),
            DioxusUiResolution(800, 450),
        ));
    });
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Dynamic cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::Srgba(bevy::color::Srgba::BLUE),
            metallic: 0.0,
            perceptual_roughness: 0.5,
            ..default()
        })),
        Transform::from_xyz(-0.3, -0.3, 0.0),
        DynamicCube,
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 10000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 100.0,
        affects_lightmapped_meshes: true,
    });

    // Main camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.5, 5.0).looking_at(Vec3::new(-0.3, -0.1, 0.0), Vec3::Y),
        Name::new("MainCamera"),
        OrbitCamera::default(),
    ));
}

fn sync_with_ui(mut fps: ResMut<FPS>, time: Res<Time>) {
    let new_fps = 1000.0 / time.delta().as_millis() as f32;
    *fps = FPS(new_fps);
}

fn animate(
    time: Res<Time>,
    mut cube_query: Query<&mut Transform, With<DynamicCube>>,
    translation_speed: Res<CubeTranslationSpeed>,
    rotation_speed: Res<CubeRotationSpeed>,
    cube_distance: Res<SignDistance>,
) {
    for mut transform in cube_query.iter_mut() {
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs() * rotation_speed.0);
        // Cube oscillates on x; offset by SignDistance so it moves toward/away from the signpost.
        transform.translation.y = (time.elapsed_secs() * translation_speed.0).sin();

        transform.translation.z = cube_distance.0;
    }
}

fn orbit_camera_system(
    mut camera_query: Query<(&mut Transform, &mut OrbitCamera), With<Camera3d>>,
    mut mouse_motion_events: MessageReader<MouseMotion>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
) {
    for (mut transform, mut orbit_camera) in camera_query.iter_mut() {
        if mouse_button_input.pressed(MouseButton::Left) {
            for mouse_motion in mouse_motion_events.read() {
                orbit_camera.yaw -= mouse_motion.delta.x * orbit_camera.sensitivity;
                orbit_camera.pitch -= mouse_motion.delta.y * orbit_camera.sensitivity;
                orbit_camera.pitch = orbit_camera.pitch.clamp(-1.5, 1.5);
            }
        }

        let yaw_quat = Quat::from_rotation_y(orbit_camera.yaw);
        let pitch_quat = Quat::from_rotation_x(orbit_camera.pitch);

        let rotation = yaw_quat * pitch_quat;
        let position = rotation * Vec3::new(0.0, 0.0, orbit_camera.distance);

        transform.translation = position;
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}
