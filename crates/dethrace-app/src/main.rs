use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use dethrace_assets::{FlicClip, GameDir};
use dethrace_ui::{MainMenuPlugin, MenuStill};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 2 || args[0] != OsStr::new("--game-dir") {
        return Err("usage: dethrace-app --game-dir PATH".into());
    }
    let game_dir = GameDir::new(PathBuf::from(&args[1]))?;
    let path = game_dir.anim_path("MAI2STIL.FLI")?;
    let clip = FlicClip::load(&path, false)?;
    let still = clip
        .frames
        .last()
        .ok_or("MAI2STIL.FLI has no frames")?
        .clone();

    App::new()
        .insert_resource(MenuStill(still))
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
