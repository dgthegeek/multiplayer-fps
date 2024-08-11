use bevy::prelude::*;
use tokio::net::UdpSocket;
use serde::{Serialize, Deserialize};
use tokio::runtime::Runtime;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;


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
    Welcome { map_size: (f32, f32), player_id: String },
    GameState { players: HashMap<String, (f32, f32)> },
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
            player_id: None,
            players: HashMap::new(),
            map_size: (0.0, 0.0),
        })
        .insert_resource(NetworkReceiver(network_receiver))
        .insert_resource(NetworkSender(client_sender))
        .add_startup_system(setup)
        .add_system(player_input)
        .add_system(handle_network_messages)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn player_input(
    keyboard_input: Res<Input<KeyCode>>,
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
        
        let move_message = ClientMessage::Move { direction };
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
            ServerMessage::Welcome { map_size, player_id } => {
                game_state.map_size = map_size;
                game_state.player_id = Some(player_id);
                println!("Joined game. Map size: {:?}", map_size);
            }
            ServerMessage::GameState { players } => {
                game_state.players = players;
                update_player_entities(&mut commands, &game_state, &mut query_set);
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
    if let Some(player_id) = &game_state.player_id {
        // Update the main player
        if let Some(&position) = game_state.players.get(player_id) {
            let mut player_query = query_set.p0();
            if let Some((_, mut transform)) = player_query.iter_mut().next() {
                transform.translation.x = position.0;
                transform.translation.y = position.1;
            } else {
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
        }

        // Update other players
        let mut other_players_query = query_set.p1();
        let mut existing_players: Vec<String> = other_players_query.iter().map(|(_, _, op)| op.name.clone()).collect();

        for (name, &position) in game_state.players.iter() {
            if name != player_id {
                if let Some((_, mut transform, _)) = other_players_query.iter_mut().find(|(_, _, op)| &op.name == name) {
                    transform.translation.x = position.0;
                    transform.translation.y = position.1;
                    existing_players.retain(|n| n != name);
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

        // Remove players that are no longer in the game
        for name in existing_players {
            if let Some((entity, _, _)) = other_players_query.iter().find(|(_, _, op)| op.name == name) {
                commands.entity(entity).despawn();
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