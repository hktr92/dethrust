//! Static end-race Damage Gallery composition at 320x200 logical resolution.

use bevy::camera::{
    ClearColorConfig, OrthographicProjection, PerspectiveProjection, Projection, ScalingMode,
    Viewport,
};
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

pub struct DamageGalleryPlugin;

impl Plugin for DamageGalleryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, show_gallery);
        app.add_systems(Update, spin_gallery);
    }
}

fn advance_spin(transform: &mut Transform, spin: &mut GallerySpin, seconds: f64) {
    if spin.automatic {
        // SpinWrecks uses 0.05 degrees per millisecond.
        spin.angle_degrees = (spin.angle_degrees + seconds * 50.0).rem_euclid(360.0);
        transform.rotation = Quat::from_rotation_y(spin.angle_degrees.to_radians() as f32);
    }
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
    commands.spawn((
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
    ));
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
    commands.spawn((
        Text2d::new(names.first().cloned().unwrap_or_default()),
        TextFont::from_font_size(8.0),
        TextColor(Color::srgb_u8(84, 254, 0)),
        logical_transform(160.0, 185.0, 4.0),
    ));
    commands.insert_resource(GalleryState {
        cars,
        names,
        selected: 0,
    });
}

#[cfg(test)]
mod tests {
    use super::{GallerySpin, advance_spin, gallery_car_transform};
    use bevy::prelude::Vec3;
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
