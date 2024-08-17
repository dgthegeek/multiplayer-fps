use bevy::prelude::*;
use crate::game_state::GameState;
use crate::player::{Player, OtherPlayer};
use crate::camera::{PlayerCamera, PlayerRotation};
pub fn setup_3d(mut commands: Commands) {
    // Ajout d'une lumière directionnelle
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: false,
            illuminance: 10000.0,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
        ..default()
    });
    // Ajout d'une lumière ambiante
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.7,
    });
    // La caméra sera ajoutée plus tard, une fois que nous aurons la position du joueur
}
pub fn render_map(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(map) = &game_state.map {
        if !game_state.map_rendered {
            for (y, row) in map.cells.iter().enumerate() {
                for (x, &is_wall) in row.iter().enumerate() {
                    if is_wall {
                        commands.spawn(PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::Box::new(1.0, 3.0, 1.0))),
                            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
                            transform: Transform::from_xyz(x as f32, 1.5, y as f32),
                            ..default()
                        });
                    } else {
                        commands.spawn(PbrBundle {
                            mesh: meshes.add(Mesh::from(shape::Plane { size: 1.0, subdivisions: 0 })),
                            material: materials.add(Color::rgb(0.3, 0.3, 0.3).into()),
                            transform: Transform::from_xyz(x as f32, 0.0, y as f32),
                            ..default()
                        });
                    }
                }
            }
            game_state.map_rendered = true;
        }
    }
}
pub fn update_player_positions(
    mut commands: Commands,
    game_state: Res<GameState>,
    player_rotation: Res<PlayerRotation>,
    mut query_set: ParamSet<(
        Query<(Entity, &mut Transform), With<Player>>,
        Query<(Entity, &mut Transform, &OtherPlayer)>,
        Query<(Entity, &mut Transform), With<PlayerCamera>>,
    )>,
    asset_server: Res<AssetServer>,
) {
    if let Some(player_id) = &game_state.player_id {
        if let Some(&(position_x, position_y, _)) = game_state.players.get(player_id) {
            let mut player_query = query_set.p0();
            let _player_entity = if let Ok((entity, mut transform)) = player_query.get_single_mut() {
                transform.translation = Vec3::new(position_x, 0.0, position_y);
                // Appliquez une rotation de 180 degrés
                transform.rotation = Quat::from_rotation_y(player_rotation.yaw + std::f32::consts::PI);
                entity
            } else {
                commands.spawn((
                    SceneBundle {
                        scene: asset_server.load("models/player/Soldier.glb#Scene0"),
                        transform: Transform::from_xyz(position_x, 0.0, position_y)
                            .with_rotation(Quat::from_rotation_y(player_rotation.yaw + std::f32::consts::PI)) // Rotation de 180 degrés
                            .with_scale(Vec3::splat(0.03)),
                        ..default()
                    },
                    Player,
                )).id()
            };
            let eye_height = 1.0; // Hauteur de la nuque
            let forward_offset = 0.01; // Léger décalage vers l'arrière
            // Mise à jour de la caméra
            let mut camera_query = query_set.p2();
            if let Ok((_, mut camera_transform)) = camera_query.get_single_mut() {
                // Calculer la nouvelle position de la caméra
                let new_camera_position = Vec3::new(
                    position_x - forward_offset * player_rotation.yaw.sin(),
                    eye_height,
                    position_y - forward_offset * player_rotation.yaw.cos()
                );
                camera_transform.translation = new_camera_position;
                // Corriger la rotation pour regarder vers l'avant
                camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, player_rotation.yaw, player_rotation.pitch, 0.0);
            } else {
                commands.spawn((
                    Camera3dBundle {
                        transform: Transform::from_xyz(
                            position_x - forward_offset * player_rotation.yaw.sin(),
                            eye_height,
                            position_y - forward_offset * player_rotation.yaw.cos(),
                        )
                        .with_rotation(Quat::from_euler(EulerRot::YXZ, player_rotation.yaw, player_rotation.pitch, 0.0)),
                        ..default()
                    },
                    PlayerCamera,
                ));
            }
        }
    }
    // Mise à jour des autres joueurs
    let mut other_players_to_remove = Vec::new();
    {
        let mut other_player_query = query_set.p1();
        for (entity, _, other_player) in other_player_query.iter_mut() {
            if let Some(&(_, _, is_alive)) = game_state.players.get(&other_player.name) {
                if !is_alive {
                    other_players_to_remove.push(entity);
                }
            } else {
                other_players_to_remove.push(entity);
            }
        }
    }
    // Suppression des joueurs morts ou qui ne sont plus dans le jeu
    for entity in other_players_to_remove {
        commands.entity(entity).despawn_recursive();
    }
    // Ajout ou mise à jour des joueurs vivants
    for (name, &(position_x, position_y, is_alive)) in game_state.players.iter() {
        if Some(name) != game_state.player_id.as_ref() && is_alive {
            let mut other_player_query = query_set.p1();
            let existing_player = other_player_query.iter_mut().find(|(_, _, op)| &op.name == name);
            
            if let Some((_entity, mut transform, _)) = existing_player {
                // Mise à jour de la position du joueur existant
                transform.translation = Vec3::new(position_x, 0.0, position_y);
                // Appliquez une rotation de 180 degrés (π radians)
                transform.rotation = Quat::from_rotation_y(player_rotation.yaw + std::f32::consts::PI);
            } else {
                // Création d'un nouveau joueur
                commands.spawn((
                    SceneBundle {
                        scene: asset_server.load("models/player/Soldier.glb#Scene0"),
                        transform: Transform::from_xyz(position_x, 0.0, position_y)
                            .with_rotation(Quat::from_rotation_y(player_rotation.yaw + std::f32::consts::PI)) // Rotation de 180 degrés
                            .with_scale(Vec3::splat(0.03)),
                        ..default()
                    },
                    OtherPlayer { name: name.clone() },
                ));
            }
        }
    }
}