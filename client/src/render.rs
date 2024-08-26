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
            let wall_mesh = meshes.add(Mesh::from(shape::Box::new(1.0, 3.0, 1.0)));
            let wall_material = materials.add(StandardMaterial {
                base_color: Color::rgb(0.8, 0.7, 0.6),
                ..default()
            });

            for (y, row) in map.cells.iter().enumerate() {
                for (x, &is_wall) in row.iter().enumerate() {
                    if is_wall {
                        commands.spawn(PbrBundle {
                            mesh: wall_mesh.clone(),
                            material: wall_material.clone(),
                            transform: Transform::from_xyz(x as f32, 1.5, y as f32),
                            ..default()
                        });
                    }
                }
            }

            // Créer le sol
            let floor_size = map.cells.len() as f32;
            commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Plane { size: floor_size, subdivisions: 1 })),
                material: materials.add(Color::rgb(0.3, 0.3, 0.3).into()),
                transform: Transform::from_xyz(floor_size / 2.0 - 0.5, 0.0, floor_size / 2.0 - 0.5),
                ..default()
            });

            game_state.map_rendered = true;
        }
    }
}

// Ajoutez ce composant
#[derive(Component)]
pub struct Walls {
    transforms: Vec<Transform>,
}

// Ajoutez ce système pour rendre les murs
pub fn render_walls(
    walls_query: Query<&Walls>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for walls in walls_query.iter() {
        for wall_transform in &walls.transforms {
            commands.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box::new(1.0, 3.0, 1.0))),
                material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
                transform: *wall_transform,
                ..default()
            });
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
        if let Some(&(position_x, position_y, _, is_alive)) = game_state.players.get(player_id) {
            if is_alive {
                let mut player_query = query_set.p0();
                let _player_entity = if let Ok((entity, mut transform)) = player_query.get_single_mut() {
                    transform.translation = Vec3::new(position_x, 0.0, position_y);
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
                let eye_height = 1.0;
                let forward_offset = 0.01;
                let mut camera_query = query_set.p2();
                if let Ok((_, mut camera_transform)) = camera_query.get_single_mut() {
                    let new_camera_position = Vec3::new(
                        position_x - forward_offset * player_rotation.yaw.sin(),
                        eye_height,
                        position_y - forward_offset * player_rotation.yaw.cos()
                    );
                    camera_transform.translation = new_camera_position;
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
    }
    let mut other_players_to_remove = Vec::new();
    {
        let mut other_player_query = query_set.p1();
        for (entity, _, other_player) in other_player_query.iter_mut() {
            if let Some(&(_, _, _, is_alive)) = game_state.players.get(&other_player.name) {
                if !is_alive {
                    other_players_to_remove.push(entity);
                }
            } else {
                other_players_to_remove.push(entity);
            }
        }
    }
    for entity in other_players_to_remove {
        commands.entity(entity).despawn_recursive();
    }
    for (name, &(position_x, position_y, rotation, is_alive)) in game_state.players.iter() {
        if Some(name) != game_state.player_id.as_ref() && is_alive {
            let mut other_player_query = query_set.p1();
            let existing_player = other_player_query.iter_mut().find(|(_, _, op)| &op.name == name);
            
            if let Some((_entity, mut transform, _)) = existing_player {
                transform.translation = Vec3::new(position_x, 0.0, position_y);
                transform.rotation = Quat::from_rotation_y(rotation);
            } else {
                commands.spawn((
                    SceneBundle {
                        scene: asset_server.load("models/player/Soldier.glb#Scene0"),
                        transform: Transform::from_xyz(position_x, 0.0, position_y)
                            .with_rotation(Quat::from_rotation_y(rotation))
                            .with_scale(Vec3::splat(0.03)),
                        ..default()
                    },
                    OtherPlayer { name: name.clone() },
                ));
            }
        }
    }
}
#[derive(Component)]
pub struct Renderable;


pub fn update_visibility(
    mut renderable_query: Query<(&mut Visibility, &GlobalTransform), With<Renderable>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    if let Ok((_camera, camera_transform)) = camera_query.get_single() {
        let camera_position = camera_transform.translation();
        let forward = camera_transform.forward();

        for (mut visibility, transform) in renderable_query.iter_mut() {
            let to_object = transform.translation() - camera_position;
            let distance = to_object.length();
            
            // Ajustez ces valeurs selon vos besoins
            let render_distance = 13.0;
            let fov_cos = 0.5; // Approximativement 60 degrés de champ de vision
            
            if distance <= render_distance && to_object.normalize().dot(forward) > fov_cos {
                *visibility = Visibility::Inherited;
            } else {
                *visibility = Visibility::Hidden;
            }
        }
    }
}