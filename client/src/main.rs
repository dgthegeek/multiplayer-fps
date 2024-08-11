use bevy::prelude::*;
use bevy::window::Window;
use bevy::input::mouse::MouseMotion;

use tokio::net::UdpSocket;
use serde::{Serialize, Deserialize};
use tokio::runtime::Runtime;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use bevy::window::CursorGrabMode;
use bevy::ecs::system::ParamSet;

const PLAYER_SPEED: f32 = 0.1;
const SHOOT_COOLDOWN: f32 = 0.5; // Temps de recharge entre les tirs

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, States)]
enum AppState {
    #[default]
    Loading,
    RenderMap,
    Playing,
    GameOver,
}

#[derive(Resource)]
struct MouseSensitivity(f32);

#[derive(Component)]
struct PlayerCamera;

#[derive(Resource)]
struct PlayerRotation {
    yaw: f32,
    pitch: f32,
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
    players: HashMap<String, (f32, f32, bool)>, 
    map: Option<Map>,
    map_rendered: bool,
    last_shoot_time: f32,
    is_alive: bool, 
    game_over_results: Option<(String, Vec<(String, u32)>)>, // (winner, scores)
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
    GameOver { winner: String, scores: Vec<(String, u32)> },
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
                game_over_results: None,
            })
            .insert_resource(NetworkReceiver(network_receiver))
            .insert_resource(NetworkSender(client_sender))
            .add_startup_system(setup_3d)
            .add_system(handle_network_messages)
            .add_system(player_input)
            .add_system(update_player_positions)
            .add_system(render_map.in_schedule(OnEnter(AppState::RenderMap)))
            .insert_resource(MouseSensitivity(0.005))
            .insert_resource(PlayerRotation { yaw: 0.0, pitch: 0.0 })
            .add_system(player_look)
            .add_startup_system(setup_fps_camera)
            .insert_resource(CursorState { captured: true })
            .add_system(toggle_cursor_capture)
            .add_system(game_over_screen.in_schedule(OnEnter(AppState::GameOver)))
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
    windows: Query<&Window>,
    network_sender: Res<NetworkSender>,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    player_rotation: Res<PlayerRotation>,
    camera_query: Query<&Transform, With<PlayerCamera>>,
) {
    if !game_state.is_alive {
        return;
    }
    
    let mut direction = Vec3::ZERO;

    if keyboard_input.pressed(KeyCode::W) {
        direction += Vec3::new(player_rotation.yaw.sin(), 0.0, player_rotation.yaw.cos());
    }
    if keyboard_input.pressed(KeyCode::S) {
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
            }
        }
    }
}

#[derive(Resource)]
struct CursorState {
    captured: bool,
}
fn setup_fps_camera(mut windows: Query<&mut Window>, cursor_state: Res<CursorState>) {
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
//pour basculer entre le jeu et dehors
fn toggle_cursor_capture(
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


fn handle_network_messages(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    network_receiver: Res<NetworkReceiver>,
    mut app_state: ResMut<NextState<AppState>>,
){
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
            ServerMessage::GameOver { winner, scores } => {
                println!("Game Over! Winner: {}", winner);
                println!("Scores:");
                for (name, score) in &scores {
                    println!("{}: {}", name, score);
                }
                game_state.game_over_results = Some((winner, scores));
                app_state.set(AppState::GameOver);
            }
        }
    }
}


fn update_player_positions(
    mut commands: Commands,
    game_state: Res<GameState>,
    player_rotation: Res<PlayerRotation>,
    mut query_set: ParamSet<(
        Query<(Entity, &mut Transform), With<Player>>,
        Query<(Entity, &mut Transform, &OtherPlayer)>,
        Query<(Entity, &mut Transform), With<PlayerCamera>>,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(player_id) = &game_state.player_id {
        if let Some(&(position_x, position_y, _)) = game_state.players.get(player_id) {
            let mut player_query = query_set.p0();
            let player_entity = if let Ok((entity, mut transform)) = player_query.get_single_mut() {
                transform.translation = Vec3::new(position_x, 1.0, position_y); // Augmentez y à 1.0 pour la hauteur des yeux
                transform.rotation = Quat::from_rotation_y(player_rotation.yaw);
                entity
            } else {
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Capsule::default())),
                        material: materials.add(Color::rgb(0.2, 0.7, 0.9).into()),
                        transform: Transform::from_xyz(position_x, 1.0, position_y),
                        ..default()
                    },
                    Player,
                )).id()
            };

            // Mise à jour de la caméra
            let mut camera_query = query_set.p2();
            if let Ok((_, mut camera_transform)) = camera_query.get_single_mut() {
                let player_pos = Vec3::new(position_x, 1.0, position_y);
                camera_transform.translation = player_pos;
                camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, player_rotation.yaw, player_rotation.pitch, 0.0);
            } else {
                commands.spawn((
                    Camera3dBundle {
                        transform: Transform::from_xyz(position_x, 1.0, position_y),
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

//gerer la rotation avec le souris
fn player_look(
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

fn game_over_screen(
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