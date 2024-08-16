use bevy::prelude::*;
use crate::game_state::GameState;

pub fn game_over_screen(
    mut commands: Commands,
    game_state: Res<GameState>,
    asset_server: Res<AssetServer>,
) {
    if let Some((winner, scores)) = &game_state.game_over_results {
        commands.spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: Color::rgba(0.0, 0.0, 0.0, 0.8).into(),
            ..default()
        }).with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                format!("Game Over!\nWinner: {}", winner),
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Medium.ttf"),
                    font_size: 40.0,
                    color: Color::WHITE,
                },
            ));
            parent.spawn(TextBundle::from_section(
                "Scores:",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Medium.ttf"),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ));
            for (name, score) in scores {
                parent.spawn(TextBundle::from_section(
                    format!("{}: {}", name, score),
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Medium.ttf"),
                        font_size: 20.0,
                        color: Color::WHITE,
                    },
                ));
            }
        });
    }
}

pub fn display_death_screen(
    mut commands: Commands,
    game_state: Res<GameState>,
    asset_server: Res<AssetServer>,
) {
    if !game_state.is_alive {
        commands.spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: Color::rgba(0.0, 0.0, 0.0, 0.8).into(),
            ..default()
        }).with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "You were killed!",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Medium.ttf"),
                    font_size: 40.0,
                    color: Color::RED,
                },
            ));
        });
    }
}