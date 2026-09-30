use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use dethrace_assets::{
    FlicClip, GameDir,
    brender::{GallerySources, PlayerCarSources, TrackSources, VisualScene},
};
use dethrace_game::drive::{MaimStreetDrivePlugin, MaimStreetDriveSource};
use dethrace_game::{TrackViewerPlugin, TrackViewerSource};
use dethrace_ui::gallery::{DamageGalleryPlugin, GalleryPresentation};
use dethrace_ui::{ButtonClip, MENU_CHOICES, MainMenuPlugin, MenuClips};

#[derive(Resource)]
struct DebugCarScene(Option<VisualScene>);

fn spawn_debug_car(
    mut commands: Commands,
    mut scene: ResMut<DebugCarScene>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let prepared = scene
        .0
        .take()
        .expect("debug car scene already used")
        .prepare(&mut images, &mut materials, &mut meshes)
        .expect("could not prepare original car assets");
    prepared
        .spawn(&mut commands)
        .expect("could not spawn original car actors");
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 1.5).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn run_debug_car(game_dir: &GameDir) -> Result<(), Box<dyn Error>> {
    let scene = VisualScene::initial_car(game_dir)?;
    App::new()
        .insert_resource(DebugCarScene(Some(scene)))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.12)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Carmageddon car visual".into(),
                resolution: WindowResolution::new(960, 600),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, spawn_debug_car)
        .run();
    Ok(())
}

fn run_damage_gallery(game_dir: &GameDir) -> Result<(), Box<dyn Error>> {
    let sources = GallerySources::load(game_dir, "Maim Street", 1)?;
    let background = FlicClip::load(&game_dir.anim_path("SUM2STIL.FLI")?, false)?
        .frames
        .into_iter()
        .last()
        .ok_or("empty SUM2STIL.FLI")?;
    App::new()
        .insert_resource(GalleryPresentation {
            background,
            sources: Some(sources),
        })
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Maim Street Damage Gallery".into(),
                resolution: WindowResolution::new(640, 400),
                resizable: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DamageGalleryPlugin)
        .run();
    Ok(())
}

fn run_maim_street(game_dir: &GameDir) -> Result<(), Box<dyn Error>> {
    let track = TrackSources::load(game_dir, "Maim Street")?;
    App::new()
        .insert_resource(TrackViewerSource(Some(track)))
        .insert_resource(ClearColor(Color::srgb(0.12, 0.14, 0.18)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Maim Street static viewer".into(),
                resolution: WindowResolution::new(960, 600),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(TrackViewerPlugin)
        .run();
    Ok(())
}

fn run_maim_street_drive(game_dir: &GameDir) -> Result<(), Box<dyn Error>> {
    let track = TrackSources::load(game_dir, "Maim Street")?;
    let player = PlayerCarSources::initial(game_dir)?;
    App::new()
        .insert_resource(MaimStreetDriveSource(Some((track, player))))
        .insert_resource(ClearColor(Color::srgb(0.12, 0.14, 0.18)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Maim Street player car".into(),
                resolution: WindowResolution::new(960, 600),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MaimStreetDrivePlugin)
        .run();
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() < 2 || args[0] != OsStr::new("--game-dir") {
        return Err(
            "usage: dethrace-app --game-dir PATH [--debug-scene car|damage-gallery-maim-street|maim-street|maim-street-drive]"
                .into(),
        );
    }
    let game_dir = GameDir::new(PathBuf::from(&args[1]))?;
    if args.len() == 4 && args[2] == OsStr::new("--debug-scene") && args[3] == OsStr::new("car") {
        return run_debug_car(&game_dir);
    }
    if args.len() == 4
        && args[2] == OsStr::new("--debug-scene")
        && args[3] == OsStr::new("damage-gallery-maim-street")
    {
        return run_damage_gallery(&game_dir);
    }
    if args.len() == 4
        && args[2] == OsStr::new("--debug-scene")
        && args[3] == OsStr::new("maim-street")
    {
        return run_maim_street(&game_dir);
    }
    if args.len() == 4
        && args[2] == OsStr::new("--debug-scene")
        && args[3] == OsStr::new("maim-street-drive")
    {
        return run_maim_street_drive(&game_dir);
    }
    if args.len() != 2 {
        return Err(
            "usage: dethrace-app --game-dir PATH [--debug-scene car|damage-gallery-maim-street|maim-street|maim-street-drive]"
                .into(),
        );
    }
    let path = game_dir.anim_path("MAI2STIL.FLI")?;
    let still_clip = FlicClip::load(&path, false)?;
    let still = still_clip
        .frames
        .last()
        .ok_or("MAI2STIL.FLI has no frames")?
        .clone();
    let opening = FlicClip::load(&game_dir.anim_path("MAI2COME.FLI")?, false)?;
    let mut buttons = Vec::new();
    for choice in &MENU_CHOICES {
        let on = FlicClip::load(&game_dir.anim_path(choice.on_file)?, true)?;
        let off = FlicClip::load(&game_dir.anim_path(choice.off_file)?, true)?;
        if on.frames.is_empty() || off.frames.is_empty() {
            return Err(format!("{} or {} has no frames", choice.on_file, choice.off_file).into());
        }
        buttons.push(ButtonClip { on, off });
    }
    let push = FlicClip::load(&game_dir.anim_path("MAINCNIN.FLI")?, true)?;
    if opening.frames.is_empty() || push.frames.is_empty() {
        return Err("menu opening or pushed-button FLIC has no frames".into());
    }

    App::new()
        .insert_resource(MenuClips {
            opening,
            still,
            buttons,
            push,
        })
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Carmageddon".into(),
                resolution: WindowResolution::new(640, 400),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MainMenuPlugin)
        .run();
    Ok(())
}
