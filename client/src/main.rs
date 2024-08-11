use bevy::prelude::*;
use bevy::window::Window;

use tokio::net::UdpSocket;
use serde::{Serialize, Deserialize};
use tokio::runtime::Runtime;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use bevy::ecs::system::ParamSet;

const PLAYER_SPEED: f32 = 0.1;
const SHOOT_COOLDOWN: f32 = 0.5; // Temps de recharge entre les tirs

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, States)]
enum AppState {
    #[default]
    Loading,
    RenderMap,
    Playing,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Map {
    cells: Vec<Vec<bool>>,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct OtherPlayer {
    name: String,
}

#[derive(Resource)]
struct GameState {
    player_name: String,
    player_id: Option<String>,
    players: HashMap<String, (f32, f32, bool)>, // Ajout de l'état is_alive
    map: Option<Map>,
    map_rendered: bool,
    last_shoot_time: f32,
    is_alive: bool, // Nouvel état pour le joueur local
}

#[derive(Resource)]
struct NetworkReceiver(Receiver<ServerMessage>);

#[derive(Resource)]
struct NetworkSender(Sender<ClientMessage>);

#[derive(Serialize, Deserialize, Clone, Debug)]
enum ClientMessage {
    Join { name: String },
    Move { direction: (f32, f32) },
    Shoot { direction: (f32, f32) },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum ServerMessage {
    Welcome { map: Map, player_id: String },
    GameState { players: HashMap<String, (f32, f32, bool)> }, // Ajout de l'état is_alive
    PlayerShot { shooter: String, target: String },
    PlayerDied { player: String },
}

fn main() {
    let mut input = String::new();
    
    println!("Enter server IP:port (e.g., 127.0.0.1:34254): ");
    std::io::stdin().read_line(&mut input).unwrap();
    let server_addr = input.trim().to_string();
    input.clear();

    println!("Enter Name: ");
    std::io::stdin().read_line(&mut input).unwrap();
    let player_name = input.trim().to_string();

    let rt = Runtime::new().unwrap();
    let (network_sender, network_receiver) = unbounded::<ServerMessage>();
    let (client_sender, client_receiver) = unbounded::<ClientMessage>();

    rt.block_on(async {
        let socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        socket.connect(&server_addr).await.unwrap();
        println!("Connected to server at {}", server_addr);
        let join_message = ClientMessage::Join { name: player_name.clone() };
        let serialized = serde_json::to_string(&join_message).unwrap();
        socket.send(serialized.as_bytes()).await.unwrap();
        println!("Join message sent to server");

        tokio::spawn(network_loop(socket, network_sender.clone(), client_receiver));
    });

        App::new()
            .add_plugins(DefaultPlugins)
            .add_state::<AppState>()
            .insert_resource(GameState { 
                player_name: player_name.clone(),
                player_id: None,
                players: HashMap::new(),
                map: None,
                map_rendered: false,
                last_shoot_time: 0.0,
                is_alive: true,
            })
            .insert_resource(NetworkReceiver(network_receiver))
            .insert_resource(NetworkSender(client_sender))
            .add_startup_system(setup_3d)
            .add_system(handle_network_messages)
            .add_system(player_input)
            .add_system(update_player_positions)
            .add_system(render_map.in_schedule(OnEnter(AppState::RenderMap)))
            .run();
}

fn setup_3d(mut commands: Commands) {
    // Ajout d'une lumière directionnelle
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 10000.0,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
        ..default()
    });

    // Ajout d'une lumière ambiante
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.2,
    });

    // La caméra sera ajoutée plus tard, une fois que nous aurons la position du joueur
}

fn render_map(
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

fn player_input(
    keyboard_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
    network_sender: Res<NetworkSender>,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    window: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    if !game_state.is_alive {
        return; // Si le joueur est mort, ne pas traiter les entrées
    }
    let mut direction = (0.0, 0.0);

    if keyboard_input.pressed(KeyCode::W) || keyboard_input.pressed(KeyCode::Up) {
        direction.1 -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::S) || keyboard_input.pressed(KeyCode::Down) {
        direction.1 += 1.0;
    }
    if keyboard_input.pressed(KeyCode::A) || keyboard_input.pressed(KeyCode::Left) {
        direction.0 -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::D) || keyboard_input.pressed(KeyCode::Right) {
        direction.0 += 1.0;
    }

    if direction != (0.0, 0.0) {
        let magnitude = ((direction.0 as f32).powi(2) + (direction.1 as f32).powi(2)).sqrt();
        direction.0 /= magnitude;
        direction.1 /= magnitude;
        
        let move_message = ClientMessage::Move { direction };
        if let Err(e) = network_sender.0.send(move_message) {
            eprintln!("Failed to send move message: {}", e);
        }

        // Séparer l'emprunt mutable et immuable
        let player_id = game_state.player_id.clone();
        if let Some(player_id) = player_id {
            if let Some(position) = game_state.players.get_mut(&player_id) {
                position.0 += direction.0 * PLAYER_SPEED;
                position.1 += direction.1 * PLAYER_SPEED;
            }
        }
    }
        // Gestion du tir
        if mouse_input.just_pressed(MouseButton::Left) {
            let current_time = time.elapsed_seconds();
            if current_time - game_state.last_shoot_time >= SHOOT_COOLDOWN {
                game_state.last_shoot_time = current_time;
                
                let window = window.single();
                if let Some(cursor_position) = window.cursor_position() {
                    if let Ok((camera, camera_transform)) = camera_query.get_single() {
                        if let Some(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
                            let world_position = ray.origin + ray.direction * 10.0; // Distance arbitraire
                            let player_position = game_state.players.get(game_state.player_id.as_ref().unwrap()).unwrap();
                            let direction = (
                                world_position.x - player_position.0,
                                world_position.z - player_position.1,
                            );
                            let magnitude = (direction.0 * direction.0 + direction.1 * direction.1).sqrt();
                            let normalized_direction = (direction.0 / magnitude, direction.1 / magnitude);
        
                            let shoot_message = ClientMessage::Shoot { direction: normalized_direction };
                            if let Err(e) = network_sender.0.send(shoot_message) {
                                eprintln!("Failed to send shoot message: {}", e);
                            }
                            println!("Player shot in direction: {:?}", normalized_direction);
                        }
                    }
                }
            }
        }
    }


fn handle_network_messages(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    network_receiver: Res<NetworkReceiver>,
) {
    for message in network_receiver.0.try_iter() {
        println!("Received message: {:?}", message);
        match message {
            ServerMessage::Welcome { map, player_id } => {
                println!("Received Welcome message with map");
                game_state.map = Some(map);
                game_state.player_id = Some(player_id);
                game_state.map_rendered = false;  // Force map re-render
                
                // Trigger map rendering
                commands.insert_resource(NextState(Some(AppState::RenderMap)));
            }
            ServerMessage::GameState { players } => {
                game_state.players = players;
            }
            ServerMessage::PlayerShot { shooter, target } => {
                if Some(target.clone()) == game_state.player_id {
                    println!("You were shot by {}!", shooter);
                } else {
                    println!("Player {} was shot by {}!", target, shooter);
                }
            }
            ServerMessage::PlayerDied { player } => {
                if Some(player.clone()) == game_state.player_id {
                    game_state.is_alive = false;
                    println!("You died!");
                } else {
                    println!("Player {} died!", player);
                }
            }
        }
    }
}


fn update_player_positions(
    mut commands: Commands,
    game_state: Res<GameState>,
    mut query_set: ParamSet<(
        Query<(Entity, &mut Transform), With<Player>>,
        Query<(Entity, &mut Transform, &OtherPlayer)>,
        Query<(Entity, &mut Transform), With<Camera3d>>,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Mise à jour du joueur principal
    if let Some(player_id) = &game_state.player_id {
        if let Some(&position) = game_state.players.get(player_id) {
            let mut player_query = query_set.p0();
            let player_entity = if let Ok((entity, mut transform)) = player_query.get_single_mut() {
                transform.translation = Vec3::new(position.0, 0.5, position.1);
                entity
            } else {
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.8 })),
                        material: materials.add(Color::rgb(0.2, 0.7, 0.9).into()),
                        transform: Transform::from_xyz(position.0, 0.5, position.1),
                        ..default()
                    },
                    Player,
                )).id()
            };

            // Mise à jour de la caméra
            let mut camera_query = query_set.p2();
            if let Ok((_, mut camera_transform)) = camera_query.get_single_mut() {
                let player_pos = Vec3::new(position.0, 0.5, position.1);
                camera_transform.translation = player_pos + Vec3::new(0.0, 20.0, 0.0); // Placez la caméra 20 unités au-dessus du joueur
                camera_transform.look_at(player_pos, Vec3::Z); // Regardez vers le bas
            } else {
                commands.spawn(Camera3dBundle {
                    transform: Transform::from_xyz(position.0, 20.0, position.1)
                        .looking_at(Vec3::new(position.0, 0.0, position.1), Vec3::Z),
                    ..default()
                });
            }
        }
    }

    // Mise à jour des autres joueurs
    let mut other_players_to_remove = Vec::new();
    {
        let mut other_player_query = query_set.p1();
        for (entity, mut transform, other_player) in other_player_query.iter_mut() {
            if let Some(&(position_x, position_y, is_alive)) = game_state.players.get(&other_player.name) {
                transform.translation = Vec3::new(position_x, 0.5, position_y);
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
        commands.entity(entity).despawn();
    }

    // Ajout des nouveaux autres joueurs
    for (name, &(position_x, position_y, is_alive)) in game_state.players.iter() {
        if Some(name) != game_state.player_id.as_ref() && is_alive {
            let other_player_query = query_set.p1();
            if !other_player_query.iter().any(|(_, _, op)| &op.name == name) {
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.8 })),
                        material: materials.add(Color::rgb(0.9, 0.2, 0.3).into()),
                        transform: Transform::from_xyz(position_x, 0.5, position_y),
                        ..default()
                    },
                    OtherPlayer { name: name.clone() },
                ));
            }
        }
    }
}

async fn network_loop(socket: UdpSocket, sender: Sender<ServerMessage>, receiver: Receiver<ClientMessage>) {
    let socket = std::sync::Arc::new(socket);
    let send_socket = std::sync::Arc::clone(&socket);

    tokio::spawn(async move {
        loop {
            if let Ok(message) = receiver.try_recv() {
                let serialized = serde_json::to_string(&message).unwrap();
                if let Err(e) = send_socket.send(serialized.as_bytes()).await {
                    eprintln!("Failed to send message: {}", e);
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    });

    let mut buf = vec![0u8; 4096]; 
    loop {
        match socket.recv(&mut buf).await {
            Ok(n) => {
                if let Ok(message) = serde_json::from_slice::<ServerMessage>(&buf[..n]) {
                    println!("Parsed server message: {:?}", message);
                    if let Err(e) = sender.send(message) {
                        eprintln!("Failed to send message to main thread: {}", e);
                    }
                } else {
                    eprintln!("Failed to parse server message");
                }
            }
            Err(e) => eprintln!("Failed to receive data: {}", e),
        }
    }
}