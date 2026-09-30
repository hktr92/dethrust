//! Static end-race Damage Gallery composition at 320x200 logical resolution.

use bevy::app::AppExit;
use bevy::camera::{
    ClearColorConfig, OrthographicProjection, PerspectiveProjection, Projection, ScalingMode,
    Viewport,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use dethrace_assets::brender::GallerySources;

pub const RENDER_X: u32 = 11;
pub const RENDER_Y: u32 = 20;
pub const RENDER_W: u32 = 298;
pub const RENDER_H: u32 = 149;

#[derive(Resource)]
pub struct GalleryPresentation {
    pub background: Image,
    pub sources: Option<GallerySources>,
}

#[derive(Resource)]
pub struct GalleryState {
    pub cars: Vec<Entity>,
    pub names: Vec<String>,
    pub selected: usize,
    pub label: Entity,
    pub camera: Entity,
    pub outline: Vec<Entity>,
    pub zoom: f32,
    pub zoom_target: bool,
    pub last_cursor: Option<Vec2>,
}

#[derive(Component)]
pub struct GalleryCar {
    pub index: usize,
}

#[derive(Component)]
struct GallerySpin {
    automatic: bool,
    angle_degrees: f64,
}

#[derive(Component)]
struct GalleryOutline;

pub struct DamageGalleryPlugin;

impl Plugin for DamageGalleryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, show_gallery);
        app.add_systems(
            Update,
            (handle_gallery_input, update_gallery_view, spin_gallery).chain(),
        );
    }
}

fn advance_spin(transform: &mut Transform, spin: &mut GallerySpin, seconds: f64) {
    if spin.automatic {
        // SpinWrecks uses 0.05 degrees per millisecond.
        spin.angle_degrees = (spin.angle_degrees + seconds * 50.0).rem_euclid(360.0);
        transform.rotation = Quat::from_rotation_y(spin.angle_degrees.to_radians() as f32);
    }
}

fn rotate_manually(transform: &mut Transform, spin: &mut GallerySpin, yaw: f32, pitch: f32) {
    spin.automatic = false;
    transform.rotation =
        (Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch) * transform.rotation)
            .normalize();
}

fn spin_gallery(time: Res<Time>, mut cars: Query<(&mut Transform, &mut GallerySpin)>) {
    for (mut transform, mut spin) in &mut cars {
        advance_spin(&mut transform, &mut spin, time.delta_secs_f64());
    }
}

pub fn gallery_car_transform(index: usize, radius: f32) -> Result<Transform, String> {
    if !radius.is_finite() || radius <= 0.0 {
        return Err("invalid gallery car radius".into());
    }
    let x = 1.5 * ((index % 3) as f32 - 1.0);
    let y = -1.2 * ((index / 3) as f32 - 0.5);
    Ok(Transform::from_xyz(x, y, 0.0).with_scale(Vec3::splat(0.47 / radius)))
}

fn logical_transform(x: f32, y: f32, z: f32) -> Transform {
    Transform::from_xyz(x - 160.0, 100.0 - y, z)
}

fn show_gallery(
    mut commands: Commands,
    mut presentation: ResMut<GalleryPresentation>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let background = images.add(presentation.background.clone());
    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            ..default()
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 320.0,
                min_height: 200.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
    commands.spawn((
        Sprite::from_image(background),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    let mid_x = RENDER_X as f32 + RENDER_W as f32 / 2.0;
    let mid_y = RENDER_Y as f32 + RENDER_H as f32 / 2.0;
    commands.spawn((
        Sprite::from_color(
            Color::srgb_u8(176, 176, 176),
            Vec2::new(RENDER_W as f32, RENDER_H as f32),
        ),
        logical_transform(mid_x, mid_y, 1.0),
    ));
    let grid = Color::srgb_u8(61, 17, 8); // render palette index 8 in DRRENDER.PAL
    let mut x = RENDER_X as f32 + 7.5;
    while x < RENDER_X as f32 + RENDER_W as f32 {
        commands.spawn((
            Sprite::from_color(grid, Vec2::new(1.0, RENDER_H as f32)),
            logical_transform(x, mid_y, 2.0),
        ));
        x += 15.0;
    }
    let mut y = RENDER_Y as f32 + 7.5;
    while y < RENDER_Y as f32 + RENDER_H as f32 {
        commands.spawn((
            Sprite::from_color(grid, Vec2::new(RENDER_W as f32, 1.0)),
            logical_transform(mid_x, y, 2.0),
        ));
        y += 15.0;
    }
    let window = windows.single().expect("gallery window");
    let physical_width = window.physical_width();
    let physical_height = window.physical_height();
    let px = |x: u32| (x as f32 * physical_width as f32 / 320.0).round() as u32;
    let py = |y: u32| (y as f32 * physical_height as f32 / 200.0).round() as u32;
    let camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 1,
                viewport: Some(Viewport {
                    physical_position: UVec2::new(px(RENDER_X), py(RENDER_Y)),
                    physical_size: UVec2::new(
                        px(RENDER_X + RENDER_W) - px(RENDER_X),
                        py(RENDER_Y + RENDER_H) - py(RENDER_Y),
                    ),
                    depth: 0.0..1.0,
                }),
                clear_color: ClearColorConfig::None,
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: 55.0_f32.to_radians(),
                aspect_ratio: 2.0,
                near: 0.01,
                far: 100.0,
                ..default()
            }),
            Transform::from_xyz(0.0, 0.0, 2.2).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id();
    let gallery = presentation
        .sources
        .take()
        .expect("gallery sources already used");
    let mut cars = Vec::with_capacity(gallery.entries.len());
    let mut names = Vec::with_capacity(gallery.entries.len());
    for (index, entry) in gallery.entries.into_iter().enumerate() {
        let prepared = entry
            .scene
            .prepare(&mut images, &mut materials, &mut meshes)
            .unwrap_or_else(|e| panic!("{}: {e}", entry.name));
        let car = commands
            .spawn((
                GalleryCar { index },
                GallerySpin {
                    automatic: true,
                    angle_degrees: 0.0,
                },
                gallery_car_transform(index, entry.radius).expect("validated radius"),
            ))
            .id();
        for root in prepared
            .spawn(&mut commands)
            .unwrap_or_else(|e| panic!("{}: {e}", entry.name))
        {
            commands.entity(car).add_child(root);
        }
        cars.push(car);
        names.push(entry.name);
    }
    commands.spawn((
        Sprite::from_color(Color::BLACK, Vec2::new(155.0, 13.0)),
        logical_transform(160.5, 184.5, 3.0),
    ));
    let label = commands
        .spawn((
            Text2d::new(names.first().cloned().unwrap_or_default()),
            TextFont::from_font_size(8.0),
            TextColor(Color::srgb_u8(84, 254, 0)),
            logical_transform(160.0, 185.0, 4.0),
        ))
        .id();
    let mut outline = Vec::new();
    for (width, height, dx, dy) in [
        (94.0, 1.0, 0.0, -35.0),
        (94.0, 1.0, 0.0, 35.0),
        (1.0, 70.0, -47.0, 0.0),
        (1.0, 70.0, 47.0, 0.0),
    ] {
        outline.push(
            commands
                .spawn((
                    GalleryOutline,
                    Sprite::from_color(Color::srgb_u8(84, 254, 0), Vec2::new(width, height)),
                    logical_transform(61.0 + dx, 57.0 + dy, 3.0),
                ))
                .id(),
        );
    }
    commands.insert_resource(GalleryState {
        cars,
        names,
        selected: 0,
        label,
        camera,
        outline,
        zoom: 0.0,
        zoom_target: false,
        last_cursor: None,
    });
}

fn navigate(selected: usize, dx: i32, dy: i32, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let columns = 3;
    let rows = count.div_ceil(columns);
    let row = (selected / columns) as i32;
    let col = (selected % columns) as i32;
    let next = ((row + dy).rem_euclid(rows as i32) as usize) * columns
        + (col + dx).rem_euclid(columns as i32) as usize;
    if next < count { next } else { selected }
}

fn grid_hit(cursor: Vec2, window: &Window, count: usize) -> Option<usize> {
    let x = cursor.x * 320.0 / window.width();
    let y = cursor.y * 200.0 / window.height();
    if !(RENDER_X as f32..(RENDER_X + RENDER_W) as f32).contains(&x)
        || !(RENDER_Y as f32..(RENDER_Y + RENDER_H) as f32).contains(&y)
    {
        return None;
    }
    let col = ((x - RENDER_X as f32) * 3.0 / RENDER_W as f32) as usize;
    let row = ((y - RENDER_Y as f32) / (RENDER_H as f32 / 2.0)) as usize;
    (row * 3 + col < count).then_some(row * 3 + col)
}

#[derive(SystemParam)]
struct GalleryInput<'w, 's> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    gamepads: Query<'w, 's, &'static Gamepad>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
}

fn handle_gallery_input(
    mut state: ResMut<GalleryState>,
    input: GalleryInput,
    time: Res<Time>,
    mut cars: Query<(&mut Transform, &mut GallerySpin), With<GalleryCar>>,
    mut labels: Query<&mut Text2d>,
    mut exit: MessageWriter<AppExit>,
) {
    let left = input.keys.just_pressed(KeyCode::ArrowLeft)
        || input
            .gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::DPadLeft));
    let right = input.keys.just_pressed(KeyCode::ArrowRight)
        || input
            .gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::DPadRight));
    let up = input.keys.just_pressed(KeyCode::ArrowUp)
        || input
            .gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::DPadUp));
    let down = input.keys.just_pressed(KeyCode::ArrowDown)
        || input
            .gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::DPadDown));
    let confirm = input.keys.just_pressed(KeyCode::Enter)
        || input.keys.just_pressed(KeyCode::Space)
        || input
            .gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::South));
    let back = input.keys.just_pressed(KeyCode::Escape)
        || input
            .gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::East));
    let mut done = false;
    let mut clicked_back = false;
    let mut clicked_car = None;
    let cursor = input.windows.single().ok().and_then(|window| {
        let cursor = window.cursor_position()?;
        let x = cursor.x * 320.0 / window.width();
        let y = cursor.y * 200.0 / window.height();
        if input.mouse.just_pressed(MouseButton::Left) {
            if y >= 174.0 {
                clicked_back = x < 86.0;
                done = x >= 247.0;
            } else if state.zoom == 0.0 {
                clicked_car = grid_hit(cursor, window, state.cars.len());
            }
        }
        Some(cursor)
    });
    if done {
        exit.write(AppExit::Success);
        return;
    }
    if back || clicked_back {
        if state.zoom > 0.0 || state.zoom_target {
            state.zoom_target = false;
        } else {
            exit.write(AppExit::Success);
        }
        return;
    }
    if state.zoom_target {
        if state.zoom >= 1.0
            && input.mouse.pressed(MouseButton::Left)
            && let (Some(previous), Some(current)) = (state.last_cursor, cursor)
        {
            let delta = current - previous;
            if delta.length_squared() > 0.0
                && let Ok((mut transform, mut spin)) = cars.get_mut(state.cars[state.selected])
            {
                rotate_manually(&mut transform, &mut spin, delta.x * 0.01, delta.y * 0.01);
            }
        }
        if state.zoom >= 1.0 {
            let yaw = (input.keys.pressed(KeyCode::KeyD) as i32
                - input.keys.pressed(KeyCode::KeyA) as i32) as f32
                * time.delta_secs()
                * 1.5;
            let pitch = (input.keys.pressed(KeyCode::KeyS) as i32
                - input.keys.pressed(KeyCode::KeyW) as i32) as f32
                * time.delta_secs()
                * 1.5;
            if (yaw != 0.0 || pitch != 0.0)
                && let Ok((mut transform, mut spin)) = cars.get_mut(state.cars[state.selected])
            {
                rotate_manually(&mut transform, &mut spin, yaw, pitch);
            }
        }
        state.last_cursor = if input.mouse.pressed(MouseButton::Left) {
            cursor
        } else {
            None
        };
        if confirm {
            state.zoom_target = false;
        }
        return;
    }
    state.last_cursor = None;
    if state.zoom > 0.0 {
        return;
    }
    let previous = state.selected;
    if left {
        state.selected = navigate(state.selected, -1, 0, state.cars.len());
    }
    if right {
        state.selected = navigate(state.selected, 1, 0, state.cars.len());
    }
    if up {
        state.selected = navigate(state.selected, 0, -1, state.cars.len());
    }
    if down {
        state.selected = navigate(state.selected, 0, 1, state.cars.len());
    }
    if let Some(index) = clicked_car {
        state.selected = index;
    }
    if state.selected != previous
        && let Ok(mut label) = labels.get_mut(state.label)
    {
        label.0 = state.names[state.selected].clone();
    }
    if confirm || clicked_car.is_some() {
        state.zoom_target = true;
    }
}

fn update_gallery_view(
    time: Res<Time>,
    mut state: ResMut<GalleryState>,
    cars: Query<&Transform, With<GalleryCar>>,
    mut ui_transforms: Query<(&mut Transform, Option<&mut Visibility>), Without<GalleryCar>>,
) {
    let target = if state.zoom_target { 1.0 } else { 0.0 };
    state.zoom = if target > state.zoom {
        (state.zoom + time.delta_secs()).min(target)
    } else {
        (state.zoom - time.delta_secs()).max(target)
    };
    let selected = cars
        .get(state.cars[state.selected])
        .expect("gallery car")
        .translation;
    if let Ok((mut camera, _)) = ui_transforms.get_mut(state.camera) {
        camera.translation = Vec3::new(
            selected.x * state.zoom,
            selected.y * state.zoom,
            2.2 - 1.45 * state.zoom,
        );
    }
    let center_x = 61.0 + (state.selected % 3) as f32 * 99.0;
    let center_y = 57.0 + (state.selected / 3) as f32 * 74.0;
    for (index, entity) in state.outline.iter().enumerate() {
        if let Ok((mut transform, Some(mut visibility))) = ui_transforms.get_mut(*entity) {
            let (dx, dy) = match index {
                0 => (0.0, -35.0),
                1 => (0.0, 35.0),
                2 => (-47.0, 0.0),
                _ => (47.0, 0.0),
            };
            transform.translation =
                logical_transform(center_x + dx, center_y + dy, 3.0).translation;
            *visibility = if state.zoom == 0.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GallerySpin, advance_spin, gallery_car_transform, grid_hit, navigate, rotate_manually,
    };
    use bevy::prelude::{Vec2, Vec3, Window};
    #[test]
    fn uses_original_three_column_layout_and_radius() {
        assert_eq!(
            gallery_car_transform(0, 0.47).unwrap().translation,
            Vec3::new(-1.5, 0.6, 0.0)
        );
        assert_eq!(
            gallery_car_transform(5, 0.94).unwrap().translation,
            Vec3::new(1.5, -0.6, 0.0)
        );
        assert_eq!(
            gallery_car_transform(5, 0.94).unwrap().scale,
            Vec3::splat(0.5)
        );
        assert!(gallery_car_transform(0, 0.0).is_err());
    }

    #[test]
    fn manual_rotation_freezes_spin_and_preserves_position_and_scale() {
        let mut transform = gallery_car_transform(1, 0.94).unwrap();
        let original = transform;
        let mut spin = GallerySpin {
            automatic: true,
            angle_degrees: 0.0,
        };
        rotate_manually(&mut transform, &mut spin, 0.5, 0.25);
        let rotated = transform.rotation;
        advance_spin(&mut transform, &mut spin, 2.0);
        assert_eq!(transform.rotation, rotated);
        assert_eq!(transform.translation, original.translation);
        assert_eq!(transform.scale, original.scale);
        assert!((transform.rotation.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn grid_navigation_and_mouse_hit_follow_six_car_layout() {
        assert_eq!(navigate(0, -1, 0, 6), 2);
        assert_eq!(navigate(5, 0, 1, 6), 2);
        let window = Window::default();
        assert_eq!(
            grid_hit(
                Vec2::new(
                    61.0 * window.width() / 320.0,
                    57.0 * window.height() / 200.0
                ),
                &window,
                6
            ),
            Some(0)
        );
        assert_eq!(
            grid_hit(
                Vec2::new(
                    260.0 * window.width() / 320.0,
                    130.0 * window.height() / 200.0
                ),
                &window,
                6
            ),
            Some(5)
        );
        assert_eq!(grid_hit(Vec2::ZERO, &window, 6), None);
    }

    #[test]
    fn spin_matches_elapsed_time_without_transform_drift() {
        let initial = gallery_car_transform(5, 0.94).unwrap();
        for steps in [10, 60, 240] {
            let mut transform = initial;
            let mut spin = GallerySpin {
                automatic: true,
                angle_degrees: 0.0,
            };
            for _ in 0..steps {
                advance_spin(&mut transform, &mut spin, 1.0 / f64::from(steps));
            }
            assert!((spin.angle_degrees - 50.0).abs() < 1e-9);
            assert_eq!(transform.translation, initial.translation);
            assert_eq!(transform.scale, initial.scale);
            assert!((transform.rotation.length() - 1.0).abs() < 1e-6);
            spin.automatic = false;
            let rotation = transform.rotation;
            advance_spin(&mut transform, &mut spin, 1.0);
            assert_eq!(transform.rotation, rotation);
        }
    }
}
