use tokio::net::UdpSocket;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::net::SocketAddr;
use std::time::Duration;
use crate::game_state::GameState;
use crate::messages::{ClientMessage, ServerMessage};
use crate::player::{Player, PLAYER_SPEED, SHOOT_RANGE};
use crate::map::is_valid_move;

pub async fn handle_message(
    message: ClientMessage,
    addr: SocketAddr,
    game_state: Arc<Mutex<GameState>>,
    socket: Arc<UdpSocket>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = game_state.lock().await;
    match message {
        ClientMessage::Join { name } => {
            println!("Player connected: {} (IP: {})", name, addr);
            let spawn_position = state.map.generate_valid_spawn_point();
            let player = Player {
                name: name.clone(),
                position: spawn_position,
                is_alive: true,
                points: 0
            };
            state.players.insert(addr, player);
            let welcome_message = ServerMessage::Welcome {
                map: state.map.clone(),
                player_id: name,
                difficulty: state.difficulty,
            };
            let serialized = serde_json::to_string(&welcome_message)?;
            socket.send_to(serialized.as_bytes(), addr).await?;
            println!("Sent Welcome message to new player");
            broadcast_game_state(&state, &socket).await?;
        }
        ClientMessage::Move { direction } => {
            let new_position = {
                if let Some(player) = state.players.get(&addr) {
                    let new_x = player.position.0 + direction.0 * PLAYER_SPEED;
                    let new_y = player.position.1 + direction.1 * PLAYER_SPEED;
                    if is_valid_move(&state.map, new_x, new_y) {
                        Some((new_x, new_y))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
        
            if let Some(new_pos) = new_position {
                if let Some(player) = state.players.get_mut(&addr) {
                    player.position = new_pos;
                }
            }
        }
        ClientMessage::Shoot { direction } => {
            let shooter = state.players.get(&addr).cloned();
            if let Some(shooter) = shooter {
                println!("Player {} is shooting!", shooter.name);
                
                let start_pos = shooter.position;
                
                let mut hit_player = None;
                let mut closest_distance = f32::MAX;
                
                for (player_addr, player) in state.players.iter() {
                    if player_addr != &addr && player.is_alive {
                        let player_pos = player.position;
                        
                        // Calculer la distance du joueur à la ligne de tir
                        let to_player = (player_pos.0 - start_pos.0, player_pos.1 - start_pos.1);
                        let dot_product = to_player.0 * direction.0 + to_player.1 * direction.1;
                        
                        if dot_product > 0.0 && dot_product < SHOOT_RANGE {
                            let closest_point = (
                                start_pos.0 + direction.0 * dot_product,
                                start_pos.1 + direction.1 * dot_product
                            );
                            
                            let distance = ((player_pos.0 - closest_point.0).powi(2) + (player_pos.1 - closest_point.1).powi(2)).sqrt();
                            
                            if distance < 0.5 { // Augmenté pour tenir compte de la taille du modèle
                                let player_distance = ((player_pos.0 - start_pos.0).powi(2) + (player_pos.1 - start_pos.1).powi(2)).sqrt();
                                if player_distance < closest_distance {
                                    closest_distance = player_distance;
                                    hit_player = Some((player_addr.clone(), player.name.clone()));
                                }
                            }
                        }
                    }
                }
                
                if let Some((hit_addr, hit_name)) = hit_player {
                    if let Some(player) = state.players.get_mut(&hit_addr) {
                        player.is_alive = false;
                    }
                    if let Some(shooter) = state.players.get_mut(&addr) {
                        shooter.points += 10;
                    }
                    
                    let shot_message = ServerMessage::PlayerShot { 
                        shooter: shooter.name.clone(),
                        target: hit_name.clone(),
                    };
                    let serialized = serde_json::to_string(&shot_message)?;
                    socket.send_to(serialized.as_bytes(), &hit_addr).await?;
                    
                    let death_message = ServerMessage::PlayerDied { 
                        player: hit_name.clone()
                    };
                    let serialized = serde_json::to_string(&death_message)?;
                    for addr in state.players.keys() {
                        socket.send_to(serialized.as_bytes(), addr).await?;
                    }
                    
                    println!("Player {} was shot and killed by {}!", hit_name, shooter.name);
                } else {
                    println!("Player {} missed their shot!", shooter.name);
                }
            }
        }
    }
    broadcast_game_state(&state, &socket).await?;
    Ok(())
}


pub async fn broadcast_game_state(
    state: &GameState,
    socket: &Arc<UdpSocket>,
) -> Result<(), Box<dyn std::error::Error>> {
    let players_state: HashMap<String, (f32, f32, bool)> = state.players
        .iter()
        .filter(|(_, player)| player.is_alive)
        .map(|(_, player)| (player.name.clone(), (player.position.0, player.position.1, player.is_alive)))
        .collect();
    let game_state_message = ServerMessage::GameState { players: players_state.clone() };
    let serialized = serde_json::to_string(&game_state_message)?;
    
    println!("Broadcasting GameState:");
    for (name, (x, y, is_alive)) in &players_state {
        println!("  Player: {}, Position: ({}, {}), Alive: {}", name, x, y, is_alive);
    }
    
    for addr in state.players.keys() {
        socket.send_to(serialized.as_bytes(), addr).await?;
    }
    Ok(())
}
pub async fn check_game_over(game_state: Arc<Mutex<GameState>>, socket: Arc<UdpSocket>) -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        let mut state = game_state.lock().await;
        if state.is_game_over() {
            let winner = state.players.values()
                .max_by_key(|p| p.points)
                .cloned();
            if let Some(winner) = winner {
                let game_over_message = ServerMessage::GameOver {
                    winner: winner.name,
                    scores: state.players.values().map(|p| (p.name.clone(), p.points)).collect(),
                };
                let serialized = serde_json::to_string(&game_over_message)?;
                for addr in state.players.keys() {
                    socket.send_to(serialized.as_bytes(), addr).await?;
                }
                // Réinitialiser le jeu
                *state = GameState::new(state.difficulty);
            }
        }
    }
}