use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::CursorGrabMode;
use crate::game_state::GameState;
use crate::camera::{MouseSensitivity, PlayerCamera, PlayerRotation};
use crate::network::NetworkSender;
use crate::messages::ClientMessage;
use crate::player::{Bullet, PLAYER_SPEED, SHOOT_COOLDOWN};

#[derive(Resource)]
pub struct CursorState {
    pub captured: bool,
}

#[derive(Resource)]
pub struct MovementTimer(pub Timer);

pub fn player_input(
    keyboard_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    windows: Query<&Window>,
    network_sender: Res<NetworkSender>,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    mut timer: ResMut<MovementTimer>, // Ajoutez cette ligne
    player_rotation: Res<PlayerRotation>,
    camera_query: Query<&Transform, With<PlayerCamera>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Mise à jour du timer
    timer.0.tick(time.delta());

    // Gestion du tir
    if mouse_input.just_pressed(MouseButton::Left) {
        let current_time = time.elapsed_seconds();
        if current_time - game_state.last_shoot_time >= SHOOT_COOLDOWN {
            game_state.last_shoot_time = current_time;

            if let Ok(camera_transform) = camera_query.get_single() {
                let shoot_direction = camera_transform.forward();
                let shoot_message = ClientMessage::Shoot { direction: (shoot_direction.x, shoot_direction.z) };
                if let Err(e) = network_sender.0.send(shoot_message) {
                    eprintln!("Failed to send shoot message: {}", e);
                }
                println!("Player shot in direction: {:?}", (shoot_direction.x, shoot_direction.z));

                let spawn_point = camera_transform.translation + shoot_direction * 2.0;
                
                // Calculer la rotation pour aligner la capsule horizontalement
                let up = Vec3::Y;
                let _right = shoot_direction.cross(up).normalize();
                let bullet_rotation = Quat::from_rotation_arc(Vec3::Z, shoot_direction);

                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Capsule { 
                            radius: 0.05, 
                            rings: 0, 
                            depth: 0.5,  
                            latitudes: 8, 
                            longitudes: 18, 
                            uv_profile: shape::CapsuleUvProfile::Uniform,
                        })),
                        material: materials.add(Color::RED.into()),
                        transform: Transform::from_translation(spawn_point)
                            .with_rotation(bullet_rotation * Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)), // Rotation pour rendre la capsule horizontale
                        ..default()
                    },
                    Bullet {
                        lifetime: Timer::from_seconds(0.2, TimerMode::Once),
                    },
                ));
            }
        }
    }

    // Ne pas traiter le mouvement si le timer n'est pas terminé
    if !timer.0.finished() {
        return;
    }

    // Gestion du mouvement
    let mut direction = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::S) {
        direction += Vec3::new(player_rotation.yaw.sin(), 0.0, player_rotation.yaw.cos());
    }
    if keyboard_input.pressed(KeyCode::W) {
        direction += Vec3::new(-player_rotation.yaw.sin(), 0.0, -player_rotation.yaw.cos());
    }
    if keyboard_input.pressed(KeyCode::A) {
        direction += Vec3::new(-player_rotation.yaw.cos(), 0.0, player_rotation.yaw.sin());
    }
    if keyboard_input.pressed(KeyCode::D) {
        direction += Vec3::new(player_rotation.yaw.cos(), 0.0, -player_rotation.yaw.sin());
    }

    if direction != Vec3::ZERO {
        direction = direction.normalize();
        let move_message = ClientMessage::Move { direction: (direction.x, direction.z) };
        if let Err(e) = network_sender.0.send(move_message) {
            eprintln!("Failed to send move message: {}", e);
        }
        let player_id = game_state.player_id.clone();
        if let Some(player_id) = player_id {
            if let Some(position) = game_state.players.get_mut(&player_id) {
                position.0 += direction.x * PLAYER_SPEED;
                position.1 += direction.z * PLAYER_SPEED;
            }
        }
    }
}


pub fn player_look(
    mut motion_evr: EventReader<MouseMotion>,
    mut player_rotation: ResMut<PlayerRotation>,
    sensitivity: Res<MouseSensitivity>,
    cursor_state: Res<CursorState>,
) {
    if cursor_state.captured {
        for ev in motion_evr.iter() {
            player_rotation.yaw -= ev.delta.x * sensitivity.0;
            player_rotation.pitch -= ev.delta.y * sensitivity.0;
        }
        player_rotation.pitch = player_rotation.pitch.clamp(-1.54, 1.54);
    }
}

pub fn toggle_cursor_capture(
    keyboard_input: Res<Input<KeyCode>>,
    mut cursor_state: ResMut<CursorState>,
    mut windows: Query<&mut Window>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        cursor_state.captured = !cursor_state.captured;
        if let Ok(mut window) = windows.get_single_mut() {
            if cursor_state.captured {
                window.cursor.grab_mode = CursorGrabMode::Locked;
                window.cursor.visible = false;
            } else {
                window.cursor.grab_mode = CursorGrabMode::None;
                window.cursor.visible = true;
            }
        }
    }
}
