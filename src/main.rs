use std::time::Duration;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::input::common_conditions::input_toggle_active;
use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier3d::prelude::*;
use time::PhysicsTimeExt;

mod camera;
mod time;
mod ui;

#[derive(Event)]
struct RestartEvent;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            WorldInspectorPlugin::new().run_if(input_toggle_active(false, KeyCode::F12)),
            RapierPhysicsPlugin::<NoUserData>::default().with_default_system_setup(false),
            //RapierDebugRenderPlugin::default(),
            camera::OrbitCameraPlugin,
            time::TimePlugin,
            ui::UiPlugin,
        ))
        .add_event::<RestartEvent>()
        .add_systems(Startup, spawn_scene)
        .add_systems(PreUpdate, reset_scene.before(time::run_physics_schedule))
        .add_systems(Update, close_on_esc)
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(LogDiagnosticsPlugin {
            wait_duration: Duration::from_millis(1000),
            filter: Some(vec![FrameTimeDiagnosticsPlugin::FPS]),
            ..default()
        })
        .add_systems(
            time::PhysicsSchedule,
            (
                RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::SyncBackend)
                    .in_set(PhysicsSet::SyncBackend),
                RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::StepSimulation)
                    .in_set(PhysicsSet::StepSimulation),
                RapierPhysicsPlugin::<NoUserData>::get_systems(PhysicsSet::Writeback)
                    .in_set(PhysicsSet::Writeback),
            )
            .chain()
            .before(TransformSystem::TransformPropagate),
        )
        .add_systems(Last, bevy_rapier3d::plugin::systems::sync_removals)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(TimestepMode::Fixed {
            dt: time::DEFAULT_TIMESTEP.as_secs_f32(),
            substeps: 1,
        })
        .run();
}

#[derive(Component)]
struct Ball;

fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventWriter<RestartEvent>,
) {
    events.send(RestartEvent);

    // circular base
    commands
        .spawn((
            Transform::default(),
            RigidBody::Fixed,
            Collider::cylinder(0.01, 7.0),
        ))
        .with_children(|commands| {
            commands.spawn((
                Mesh3d(meshes.add(Circle::new(7.0))),
                MeshMaterial3d(materials.add(Color::WHITE)),
                Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ));
        });

    // light
    commands.spawn((
        PointLight {
            intensity: 1_200_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // camera
    let expected_transform = Transform::from_xyz(-5., 7.5, 16.).looking_at(Vec3::Y, Vec3::Y);
    let (yaw, pitch, _roll) = expected_transform.rotation.to_euler(EulerRot::YXZ);
    commands.spawn(camera::OrbitCameraBundle {
        orbit_camera: camera::OrbitCamera {
            gimbal_x: -yaw,
            gimbal_y: -pitch,
            distance: expected_transform.translation.length(),
            ..default()
        },
        ..default()
    });
}

fn reset_scene(
    mut commands: Commands,
    mut time: ResMut<time::PhysicsTime>,
    mut events: EventReader<RestartEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut balls: Query<Entity, With<Ball>>,
) {
    if events.is_empty() {
        return;
    }
    events.clear();

    *time = time::PhysicsTime::default();
    time.resume();

    for entity in balls.iter_mut() {
        commands.entity(entity).despawn();
    }

    // Falling ball
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5).mesh().uv(32, 18))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0., 4., 0.),
        RigidBody::Dynamic,
        Collider::ball(0.5),
        Restitution::coefficient(0.9),
        Ball,
    ));
}

pub fn close_on_esc(
    mut commands: Commands,
    focused_windows: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for (window, focus) in focused_windows.iter() {
        if !focus.focused {
            continue;
        }

        if input.just_pressed(KeyCode::Escape) {
            commands.entity(window).despawn();
        }
    }
}
