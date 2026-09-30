//! Presentation of the original Carmageddon menu.

use std::time::Duration;

use bevy::camera::{OrthographicProjection, Projection, ScalingMode};
use bevy::ecs::system::SystemParam;
use bevy::image::Image;
use bevy::input::gamepad::{Gamepad, GamepadButton};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use dethrace_assets::FlicClip;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    MainMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    NewGame,
    NetworkGame,
    Options,
    LoadGame,
    Quit,
}

impl MenuAction {
    fn label(self) -> &'static str {
        match self {
            Self::NewGame => "New Game",
            Self::NetworkGame => "New Network Game",
            Self::Options => "Options",
            Self::LoadGame => "Load Game",
            Self::Quit => "Quit Carmageddon",
        }
    }
}

pub struct MenuChoice {
    pub action: MenuAction,
    pub on_file: &'static str,
    pub off_file: &'static str,
    pub x: i32,
    pub y: i32,
    pub hit_left: i32,
    pub hit_top: i32,
    pub hit_bottom: i32,
    pub push_x: i32,
    pub push_y: i32,
}

pub const MENU_CHOICES: [MenuChoice; 5] = [
    MenuChoice {
        action: MenuAction::NewGame,
        on_file: "MAI2N1GL.FLI",
        off_file: "MAI2N1FL.FLI",
        x: 58,
        y: 51,
        hit_left: 57,
        hit_top: 51,
        hit_bottom: 61,
        push_x: 56,
        push_y: 50,
    },
    MenuChoice {
        action: MenuAction::NetworkGame,
        on_file: "MAI2NNGL.FLI",
        off_file: "MAI2NNFL.FLI",
        x: 53,
        y: 74,
        hit_left: 52,
        hit_top: 74,
        hit_bottom: 84,
        push_x: 51,
        push_y: 73,
    },
    MenuChoice {
        action: MenuAction::Options,
        on_file: "MAI2OPGL.FLI",
        off_file: "MAI2OPFL.FLI",
        x: 51,
        y: 96,
        hit_left: 51,
        hit_top: 96,
        hit_bottom: 106,
        push_x: 50,
        push_y: 95,
    },
    MenuChoice {
        action: MenuAction::LoadGame,
        on_file: "MAI2LDGL.FLI",
        off_file: "MAI2LDFL.FLI",
        x: 52,
        y: 119,
        hit_left: 52,
        hit_top: 119,
        hit_bottom: 129,
        push_x: 51,
        push_y: 118,
    },
    MenuChoice {
        action: MenuAction::Quit,
        on_file: "MAI2QTGL.FLI",
        off_file: "MAI2QTFL.FLI",
        x: 58,
        y: 142,
        hit_left: 58,
        hit_top: 142,
        hit_bottom: 152,
        push_x: 57,
        push_y: 141,
    },
];

pub struct ButtonClip {
    pub on: FlicClip,
    pub off: FlicClip,
}

#[derive(Resource)]
pub struct MenuClips {
    pub opening: FlicClip,
    pub still: Image,
    pub buttons: Vec<ButtonClip>,
    pub push: FlicClip,
}

struct Frames {
    handles: Vec<Handle<Image>>,
    period: Duration,
    size: UVec2,
}

impl Frames {
    fn from_clip(clip: &mut FlicClip, images: &mut Assets<Image>) -> Self {
        let first = &clip.frames[0];
        let size = UVec2::new(first.width(), first.height());
        Self {
            handles: std::mem::take(&mut clip.frames)
                .into_iter()
                .map(|image| images.add(image))
                .collect(),
            period: Duration::from_millis(u64::from(clip.frame_delay_ms)),
            size,
        }
    }
}

struct ButtonFrames {
    on: Frames,
    off: Frames,
}

#[derive(Component)]
struct MenuBase;

#[derive(Component)]
struct MenuButton {
    index: usize,
    animation: Option<ButtonAnimation>,
}

#[derive(Component)]
struct MenuStub;

struct ButtonAnimation {
    frames: Vec<Handle<Image>>,
    period: Duration,
    elapsed: Duration,
    next_frame: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Opening,
    Ready,
    Pushing(MenuAction),
    Stub(MenuAction),
    QuitConfirm,
}

#[derive(Resource)]
struct MenuPlayback {
    opening: Frames,
    still: Handle<Image>,
    buttons: Vec<ButtonFrames>,
    push: Frames,
    elapsed: Duration,
    next_frame: usize,
    phase: Phase,
    selected: usize,
    initial_pending: bool,
    last_cursor: Option<Vec2>,
}

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(OnEnter(AppState::MainMenu), show_main_menu)
            .add_systems(
                Update,
                (advance_opening, animate_button_images, handle_input)
                    .chain()
                    .run_if(in_state(AppState::MainMenu)),
            );
    }
}

fn show_main_menu(
    mut commands: Commands,
    mut clips: ResMut<MenuClips>,
    mut images: ResMut<Assets<Image>>,
) {
    let opening = Frames::from_clip(&mut clips.opening, &mut images);
    let still = images.add(clips.still.clone());
    let buttons = clips
        .buttons
        .iter_mut()
        .map(|button| ButtonFrames {
            on: Frames::from_clip(&mut button.on, &mut images),
            off: Frames::from_clip(&mut button.off, &mut images),
        })
        .collect::<Vec<_>>();
    let push = Frames::from_clip(&mut clips.push, &mut images);

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
    commands.spawn((MenuBase, Sprite::from_image(opening.handles[0].clone())));
    for (index, choice) in MENU_CHOICES.iter().enumerate() {
        commands.spawn((
            MenuButton {
                index,
                animation: None,
            },
            Sprite::from_image(buttons[index].on.handles[0].clone()),
            menu_transform(choice.x, choice.y, buttons[index].on.size),
            Visibility::Hidden,
        ));
    }
    commands.insert_resource(MenuPlayback {
        opening,
        still,
        buttons,
        push,
        elapsed: Duration::ZERO,
        next_frame: 1,
        phase: Phase::Opening,
        selected: 0,
        initial_pending: false,
        last_cursor: None,
    });
}

fn menu_transform(x: i32, y: i32, size: UVec2) -> Transform {
    Transform::from_xyz(
        x as f32 + size.x as f32 / 2.0 - 160.0,
        100.0 - y as f32 - size.y as f32 / 2.0,
        1.0,
    )
}

fn advance_opening(
    time: Res<Time>,
    mut playback: Option<ResMut<MenuPlayback>>,
    mut base: Query<&mut Sprite, With<MenuBase>>,
) {
    let Some(ref mut playback) = playback else {
        return;
    };
    if playback.phase != Phase::Opening {
        return;
    }
    playback.elapsed += time.delta();
    let period = playback.opening.period;
    while playback.elapsed >= period {
        playback.elapsed -= period;
        if playback.next_frame < playback.opening.handles.len() {
            base.single_mut().unwrap().image =
                playback.opening.handles[playback.next_frame].clone();
            playback.next_frame += 1;
        } else {
            base.single_mut().unwrap().image = playback.still.clone();
            playback.phase = Phase::Ready;
            playback.initial_pending = true;
            break;
        }
    }
}

fn start_button(
    buttons: &mut Query<(
        &mut MenuButton,
        &mut Sprite,
        &mut Visibility,
        &mut Transform,
    )>,
    index: usize,
    frames: &Frames,
    x: i32,
    y: i32,
) {
    for (mut button, mut sprite, mut visibility, mut transform) in buttons.iter_mut() {
        if button.index == index {
            sprite.image = frames.handles[0].clone();
            *visibility = Visibility::Visible;
            *transform = menu_transform(x, y, frames.size);
            button.animation = Some(ButtonAnimation {
                frames: frames.handles.clone(),
                period: frames.period,
                elapsed: Duration::ZERO,
                next_frame: 1,
            });
            break;
        }
    }
}

fn animate_button_images(mut buttons: Query<(&mut MenuButton, &mut Sprite)>, time: Res<Time>) {
    for (mut button, mut sprite) in &mut buttons {
        let Some(animation) = button.animation.as_mut() else {
            continue;
        };
        animation.elapsed += time.delta();
        let mut finished = false;
        while animation.elapsed >= animation.period {
            animation.elapsed -= animation.period;
            if animation.next_frame < animation.frames.len() {
                sprite.image = animation.frames[animation.next_frame].clone();
                animation.next_frame += 1;
            } else {
                finished = true;
                break;
            }
        }
        if finished {
            button.animation = None;
        }
    }
}

fn hit_test(x: f32, y: f32) -> Option<usize> {
    MENU_CHOICES.iter().position(|choice| {
        x >= choice.hit_left as f32
            && x <= 265.0
            && y >= choice.hit_top as f32
            && y <= choice.hit_bottom as f32
    })
}

fn show_stub(commands: &mut Commands, text: &str) {
    commands.spawn((
        MenuStub,
        Sprite::from_color(Color::BLACK, Vec2::new(320.0, 200.0)),
        Transform::from_xyz(0.0, 0.0, 2.0),
    ));
    commands.spawn((
        MenuStub,
        Text2d::new(text),
        TextFont::from_font_size(12.0),
        TextColor(Color::WHITE),
        Transform::from_xyz(0.0, 0.0, 3.0),
    ));
}

#[derive(SystemParam)]
struct MenuControls<'w, 's> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    gamepads: Query<'w, 's, &'static Gamepad>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    camera: Query<'w, 's, (&'static Camera, &'static GlobalTransform)>,
}

fn handle_input(
    mut commands: Commands,
    mut playback: Option<ResMut<MenuPlayback>>,
    input: MenuControls,
    mut buttons: Query<(
        &mut MenuButton,
        &mut Sprite,
        &mut Visibility,
        &mut Transform,
    )>,
    stubs: Query<Entity, With<MenuStub>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(ref mut playback) = playback else {
        return;
    };
    if playback.initial_pending {
        let choice = &MENU_CHOICES[0];
        start_button(&mut buttons, 0, &playback.buttons[0].on, choice.x, choice.y);
        playback.initial_pending = false;
    }
    if playback.phase == Phase::Opening {
        return;
    }
    let up = input.keys.just_pressed(KeyCode::ArrowUp)
        || input.keys.just_pressed(KeyCode::Numpad8)
        || input
            .gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::DPadUp));
    let down = input.keys.just_pressed(KeyCode::ArrowDown)
        || input.keys.just_pressed(KeyCode::Numpad2)
        || input
            .gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::DPadDown));
    let confirm = input.keys.just_pressed(KeyCode::Enter)
        || input.keys.just_pressed(KeyCode::NumpadEnter)
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

    match playback.phase {
        Phase::Stub(_) | Phase::QuitConfirm => {
            if playback.phase == Phase::QuitConfirm && confirm {
                exit.write(AppExit::Success);
            } else if back {
                for entity in &stubs {
                    commands.entity(entity).despawn();
                }
                playback.phase = Phase::Ready;
                let index = playback.selected;
                let choice = &MENU_CHOICES[index];
                start_button(
                    &mut buttons,
                    index,
                    &playback.buttons[index].on,
                    choice.x,
                    choice.y,
                );
            }
            return;
        }
        Phase::Pushing(action) => {
            let finished = buttons
                .iter_mut()
                .find(|(button, ..)| button.index == playback.selected)
                .is_some_and(|(button, ..)| button.animation.is_none());
            if finished {
                if action == MenuAction::Quit {
                    show_stub(&mut commands, "Quit Carmageddon?\nEnter: quit\nEsc: stay");
                    playback.phase = Phase::QuitConfirm;
                } else {
                    show_stub(
                        &mut commands,
                        &format!(
                            "{}\nNot implemented yet\nEsc: return to menu",
                            action.label()
                        ),
                    );
                    playback.phase = Phase::Stub(action);
                }
            }
            return;
        }
        Phase::Ready => {}
        Phase::Opening => return,
    }
    if back {
        show_stub(&mut commands, "Quit Carmageddon?\nEnter: quit\nEsc: stay");
        playback.phase = Phase::QuitConfirm;
        return;
    }

    let mut next = playback.selected;
    if up {
        next = (next + MENU_CHOICES.len() - 1) % MENU_CHOICES.len();
    }
    if down {
        next = (next + 1) % MENU_CHOICES.len();
    }
    let mut hovered = None;
    if let (Ok(window), Ok((camera, transform))) = (input.windows.single(), input.camera.single()) {
        if let Some(cursor) = window.cursor_position() {
            if (playback.last_cursor != Some(cursor) || input.mouse.just_pressed(MouseButton::Left))
                && let Ok(world) = camera.viewport_to_world_2d(transform, cursor)
            {
                hovered = hit_test(world.x + 160.0, 100.0 - world.y);
                if let Some(index) = hovered {
                    next = index;
                }
            }
            playback.last_cursor = Some(cursor);
        } else {
            playback.last_cursor = None;
        }
    }
    if next != playback.selected {
        let old = playback.selected;
        let old_choice = &MENU_CHOICES[old];
        start_button(
            &mut buttons,
            old,
            &playback.buttons[old].off,
            old_choice.x,
            old_choice.y,
        );
        let choice = &MENU_CHOICES[next];
        start_button(
            &mut buttons,
            next,
            &playback.buttons[next].on,
            choice.x,
            choice.y,
        );
        playback.selected = next;
    }
    if confirm || (input.mouse.just_pressed(MouseButton::Left) && hovered.is_some()) {
        let index = playback.selected;
        let choice = &MENU_CHOICES[index];
        start_button(
            &mut buttons,
            index,
            &playback.push,
            choice.push_x,
            choice.push_y,
        );
        playback.phase = Phase::Pushing(choice.action);
    }
}

#[cfg(test)]
mod tests {
    use super::hit_test;

    #[test]
    fn mouse_regions_match_fresh_boot_choices() {
        assert_eq!(hit_test(57.0, 51.0), Some(0));
        assert_eq!(hit_test(265.0, 84.0), Some(1));
        assert_eq!(hit_test(100.0, 101.0), Some(2));
        assert_eq!(hit_test(100.0, 125.0), Some(3));
        assert_eq!(hit_test(100.0, 150.0), Some(4));
        assert_eq!(hit_test(266.0, 51.0), None);
    }
}
