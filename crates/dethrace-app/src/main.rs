use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use dethrace_assets::{FlicClip, GameDir};
use dethrace_ui::{ButtonClip, MENU_CHOICES, MainMenuPlugin, MenuClips};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 2 || args[0] != OsStr::new("--game-dir") {
        return Err("usage: dethrace-app --game-dir PATH".into());
    }
    let game_dir = GameDir::new(PathBuf::from(&args[1]))?;
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
