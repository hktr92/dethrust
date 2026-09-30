//! Static original-data scene inspection. Driving and simulation come later.

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use dethrace_assets::brender::TrackSources;

#[derive(Resource)]
pub struct TrackViewerSource(pub Option<TrackSources>);

#[derive(Component)]
struct FreeFlyCamera {
    yaw: f32,
    pitch: f32,
    last_cursor: Option<Vec2>,
}

pub struct TrackViewerPlugin;

impl Plugin for TrackViewerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_track)
            .add_systems(Update, inspect_track);
    }
}

fn spawn_track(
    mut commands: Commands,
    mut source: ResMut<TrackViewerSource>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let track = source.0.take().expect("track viewer source already used");
    let start = Vec3::from_array(track.spec.start_position) + Vec3::Y * 3.0;
    let yaw = track.spec.start_yaw_degrees.to_radians();
    track
        .scene
        .prepare(&mut images, &mut materials, &mut meshes)
        .expect("could not prepare Maim Street visuals")
        .spawn(&mut commands)
        .expect("could not spawn Maim Street actors");
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            near: 0.1,
            far: 2000.0,
            ..default()
        }),
        Transform::from_translation(start).with_rotation(Quat::from_rotation_y(yaw)),
        FreeFlyCamera {
            yaw,
            pitch: 0.0,
            last_cursor: None,
        },
    ));
}

fn inspect_track(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut camera: Query<(&mut Transform, &mut FreeFlyCamera)>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
        return;
    }
    let Ok((mut transform, mut fly)) = camera.single_mut() else {
        return;
    };
    let cursor = windows.single().ok().and_then(Window::cursor_position);
    if mouse.pressed(MouseButton::Right) {
        if let (Some(previous), Some(current)) = (fly.last_cursor, cursor) {
            let delta = current - previous;
            fly.yaw -= delta.x * 0.005;
            fly.pitch = (fly.pitch - delta.y * 0.005).clamp(-1.5, 1.5);
        }
        fly.last_cursor = cursor;
    } else {
        fly.last_cursor = None;
    }
    transform.rotation = Quat::from_rotation_y(fly.yaw) * Quat::from_rotation_x(fly.pitch);
    let axis = Vec3::new(
        (keys.pressed(KeyCode::KeyD) as i32 - keys.pressed(KeyCode::KeyA) as i32) as f32,
        (keys.pressed(KeyCode::Space) as i32 - keys.pressed(KeyCode::KeyQ) as i32) as f32,
        (keys.pressed(KeyCode::KeyS) as i32 - keys.pressed(KeyCode::KeyW) as i32) as f32,
    );
    let speed = if keys.pressed(KeyCode::ControlLeft) {
        80.0
    } else {
        20.0
    };
    let motion = transform.rotation * axis.normalize_or_zero() * speed * time.delta_secs();
    transform.translation += motion;
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    #[test]
    fn start_yaw_faces_positive_source_z() {
        let forward = Quat::from_rotation_y(180.0_f32.to_radians()) * -Vec3::Z;
        assert!((forward - Vec3::Z).length() < 1e-6);
    }
}
