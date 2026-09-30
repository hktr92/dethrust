//! Presentation of the original Carmageddon menu.

use bevy::camera::{OrthographicProjection, Projection, ScalingMode};
use bevy::image::Image;
use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    MainMenu,
}

#[derive(Resource)]
pub struct MenuStill(pub Image);

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(OnEnter(AppState::MainMenu), show_main_menu);
    }
}

fn show_main_menu(
    mut commands: Commands,
    still: Res<MenuStill>,
    mut images: ResMut<Assets<Image>>,
) {
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
    commands.spawn(Sprite::from_image(images.add(still.0.clone())));
}
