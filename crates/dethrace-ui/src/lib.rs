//! Presentation of the original Carmageddon menu.

use std::time::Duration;

use bevy::camera::{OrthographicProjection, Projection, ScalingMode};
use bevy::image::Image;
use bevy::prelude::*;
use dethrace_assets::FlicClip;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    MainMenu,
}

#[derive(Resource)]
pub struct MenuClips {
    pub opening: FlicClip,
    pub still: Image,
    pub initial_highlight: FlicClip,
}

#[derive(Component)]
struct MenuBase;

#[derive(Component)]
struct MenuHighlight;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Opening,
    Highlight,
    Stable,
}

#[derive(Resource)]
struct MenuPlayback {
    opening: Vec<Handle<Image>>,
    still: Handle<Image>,
    highlight: Vec<Handle<Image>>,
    opening_period: Duration,
    highlight_period: Duration,
    elapsed: Duration,
    next_frame: usize,
    phase: Phase,
}

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(OnEnter(AppState::MainMenu), show_main_menu)
            .add_systems(Update, play_main_menu.run_if(in_state(AppState::MainMenu)));
    }
}

fn show_main_menu(
    mut commands: Commands,
    mut clips: ResMut<MenuClips>,
    mut images: ResMut<Assets<Image>>,
) {
    let opening: Vec<_> = std::mem::take(&mut clips.opening.frames)
        .into_iter()
        .map(|frame| images.add(frame))
        .collect();
    let still = images.add(clips.still.clone());
    let highlight: Vec<_> = std::mem::take(&mut clips.initial_highlight.frames)
        .into_iter()
        .map(|frame| images.add(frame))
        .collect();

    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 320.0,
                min_height: 200.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
    commands.spawn((MenuBase, Sprite::from_image(opening[0].clone())));
    commands.spawn((
        MenuHighlight,
        Sprite::from_image(highlight[0].clone()),
        Transform::from_xyz(6.5, 44.5, 1.0),
        Visibility::Hidden,
    ));
    commands.insert_resource(MenuPlayback {
        opening,
        still,
        highlight,
        opening_period: Duration::from_millis(u64::from(clips.opening.frame_delay_ms)),
        highlight_period: Duration::from_millis(u64::from(clips.initial_highlight.frame_delay_ms)),
        elapsed: Duration::ZERO,
        next_frame: 1,
        phase: Phase::Opening,
    });
}

fn play_main_menu(
    time: Res<Time>,
    mut playback: Option<ResMut<MenuPlayback>>,
    mut base: Query<&mut Sprite, (With<MenuBase>, Without<MenuHighlight>)>,
    mut overlay: Query<(&mut Sprite, &mut Visibility), With<MenuHighlight>>,
) {
    let Some(ref mut playback) = playback else {
        return;
    };
    playback.elapsed += time.delta();
    loop {
        let period = match playback.phase {
            Phase::Opening => playback.opening_period,
            Phase::Highlight => playback.highlight_period,
            Phase::Stable => break,
        };
        if playback.elapsed < period {
            break;
        }
        playback.elapsed -= period;
        match playback.phase {
            Phase::Opening if playback.next_frame < playback.opening.len() => {
                base.single_mut().unwrap().image = playback.opening[playback.next_frame].clone();
                playback.next_frame += 1;
            }
            Phase::Opening => {
                base.single_mut().unwrap().image = playback.still.clone();
                playback.phase = Phase::Highlight;
                playback.next_frame = 0;
            }
            Phase::Highlight if playback.next_frame < playback.highlight.len() => {
                let (mut sprite, mut visibility) = overlay.single_mut().unwrap();
                sprite.image = playback.highlight[playback.next_frame].clone();
                *visibility = Visibility::Visible;
                playback.next_frame += 1;
            }
            Phase::Highlight => playback.phase = Phase::Stable,
            Phase::Stable => break,
        }
    }
}
