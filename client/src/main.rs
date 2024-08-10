use bevy::prelude::*;
use tokio::net::UdpSocket;
use serde::{Serialize, Deserialize};
use tokio::runtime::Runtime;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;

const MOVE_SPEED: f32 = 200.0;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct OtherPlayer {
    name: String,
}

#[derive(Resource)]
struct GameState {
    player_name: String,
    players: HashMap<String, (f32, f32)>,
    map_size: (f32, f32),
}

#[derive(Resource)]
struct NetworkReceiver(Receiver<ServerMessage>);

#[derive(Resource)]
struct NetworkSender(Sender<ClientMessage>);

#[derive(Serialize, Deserialize, Clone, Debug)]
enum ClientMessage {
    Join { name: String },
    Move { direction: (f32, f32) },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
enum ServerMessage {
    GameState { players: HashMap<String, (f32, f32)> },
    MapInfo { size: (f32, f32) },
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

        let join_message = ClientMessage::Join { name: player_name.clone() };
        let serialized = serde_json::to_string(&join_message).unwrap();
        socket.send(serialized.as_bytes()).await.unwrap();

        tokio::spawn(network_loop(socket, network_sender.clone(), client_receiver));
    });

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(GameState { 
            player_name: player_name.clone(),
            players: HashMap::new(),
            map_size: (0.0, 0.0),
        })
        .insert_resource(NetworkReceiver(network_receiver))
        .insert_resource(NetworkSender(client_sender))
        .add_startup_system(setup)
        .add_system(player_input)
        .add_system(handle_network_messages)
        .add_system(update_player_positions)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn player_input(
    keyboard_input: Res<Input<KeyCode>>,
    time: Res<Time>,
    network_sender: Res<NetworkSender>,
) {
    let mut direction = (0.0, 0.0);

    if keyboard_input.pressed(KeyCode::W) || keyboard_input.pressed(KeyCode::Up) {
        direction.1 += 1.0;
    }
    if keyboard_input.pressed(KeyCode::S) || keyboard_input.pressed(KeyCode::Down) {
        direction.1 -= 1.0;
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
        
        let movement = (
            direction.0 * MOVE_SPEED * time.delta_seconds(),
            direction.1 * MOVE_SPEED * time.delta_seconds()
        );
        
        let move_message = ClientMessage::Move { direction: movement };
        if let Err(e) = network_sender.0.send(move_message) {
            eprintln!("Failed to send move message: {}", e);
        }
    }
}

fn handle_network_messages(
    mut game_state: ResMut<GameState>,
    network_receiver: Res<NetworkReceiver>,
    mut commands: Commands,
    mut query_set: ParamSet<(
        Query<(Entity, &mut Transform), With<Player>>,
        Query<(Entity, &mut Transform, &mut OtherPlayer)>,
    )>,
) {
    for message in network_receiver.0.try_iter() {
        match message {
            ServerMessage::GameState { players } => {
                game_state.players = players;
                update_player_entities(&mut commands, &game_state, &mut query_set);
            }
            ServerMessage::MapInfo { size } => {
                game_state.map_size = size;
                println!("Received map info: {:?}", size);
            }
        }
    }
}

fn update_player_entities(
    commands: &mut Commands,
    game_state: &GameState,
    query_set: &mut ParamSet<(
        Query<(Entity, &mut Transform), With<Player>>,
        Query<(Entity, &mut Transform, &mut OtherPlayer)>,
    )>,
) {
    let mut existing_players = HashMap::new();
    for (entity, _, other_player) in query_set.p1().iter() {
        existing_players.insert(other_player.name.clone(), entity);
    }

    for (name, &position) in game_state.players.iter() {
        if name == &game_state.player_name {
            if query_set.p0().is_empty() {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgb(0.25, 0.25, 0.75),
                            custom_size: Some(Vec2::new(50.0, 50.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(Vec3::new(position.0, position.1, 0.0)),
                        ..default()
                    },
                    Player,
                ));
            }
        } else {
            if let Some(&entity) = existing_players.get(name) {
                if let Ok((_, mut transform, _)) = query_set.p1().get_mut(entity) {
                    transform.translation.x = position.0;
                    transform.translation.y = position.1;
                }
            } else {
                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgb(0.75, 0.25, 0.25),
                            custom_size: Some(Vec2::new(50.0, 50.0)),
                            ..default()
                        },
                        transform: Transform::from_translation(Vec3::new(position.0, position.1, 0.0)),
                        ..default()
                    },
                    OtherPlayer { name: name.clone() },
                ));
            }
        }
    }

    for (name, entity) in existing_players.iter() {
        if !game_state.players.contains_key(name) {
            commands.entity(*entity).despawn();
        }
    }
}

fn update_player_positions(
    game_state: Res<GameState>,
    mut query_set: ParamSet<(
        Query<&mut Transform, With<Player>>,
        Query<(&mut Transform, &OtherPlayer)>,
    )>,
) {
    if let Some(&position) = game_state.players.get(&game_state.player_name) {
        for mut transform in query_set.p0().iter_mut() {
            transform.translation.x = position.0;
            transform.translation.y = position.1;
        }
    }

    for (mut transform, other_player) in query_set.p1().iter_mut() {
        if let Some(&position) = game_state.players.get(&other_player.name) {
            transform.translation.x = position.0;
            transform.translation.y = position.1;
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

    let mut buf = [0u8; 1024];
    loop {
        match socket.recv(&mut buf).await {
            Ok(n) => {
                if let Ok(message) = serde_json::from_slice::<ServerMessage>(&buf[..n]) {
                    if let Err(e) = sender.send(message) {
                        eprintln!("Failed to send message to main thread: {}", e);
                    }
                }
            }
            Err(e) => eprintln!("Failed to receive data: {}", e),
        }
    }
}