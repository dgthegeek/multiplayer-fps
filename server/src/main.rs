use tokio::net::UdpSocket;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

const MAP_SIZE: (f32, f32) = (1000.0, 1000.0);
const PLAYER_SPEED: f32 = 5.0;

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Player {
    name: String,
    position: (f32, f32),
}

#[derive(Serialize, Deserialize, Debug)]
enum ClientMessage {
    Join { name: String },
    Move { direction: (f32, f32) },
}

#[derive(Serialize, Deserialize, Debug)]
enum ServerMessage {
    Welcome { map_size: (f32, f32), player_id: String },
    GameState { players: HashMap<String, (f32, f32)> },
}

struct GameState {
    players: HashMap<SocketAddr, Player>,
    map_size: (f32, f32),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:34254").await?;
    let socket = Arc::new(socket);
    let game_state = Arc::new(Mutex::new(GameState {
        players: HashMap::new(),
        map_size: MAP_SIZE,
    }));

    println!("Server listening on {}", socket.local_addr()?);

    loop {
        let mut buf = [0u8; 1024];
        let (len, addr) = socket.recv_from(&mut buf).await?;
        let message: ClientMessage = serde_json::from_slice(&buf[..len])?;

        handle_message(message, addr, Arc::clone(&game_state), Arc::clone(&socket)).await?;
    }
}

async fn handle_message(
    message: ClientMessage,
    addr: SocketAddr,
    game_state: Arc<Mutex<GameState>>,
    socket: Arc<UdpSocket>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut state = game_state.lock().await;
    
    match message {
        ClientMessage::Join { name } => {
            let player = Player {
                name: name.clone(),
                position: (0.0, 0.0),
            };
            state.players.insert(addr, player);
            
            let welcome_message = ServerMessage::Welcome {
                map_size: state.map_size,
                player_id: name,
            };
            let serialized = serde_json::to_string(&welcome_message)?;
            socket.send_to(serialized.as_bytes(), addr).await?;

            broadcast_game_state(&state, &socket).await?;
        }
        ClientMessage::Move { direction } => {
            // Calculez les nouvelles coordonnées du joueur avant d'emprunter `state` de manière mutable.
            let new_position = {
                if let Some(player) = state.players.get(&addr) {
                    let new_x = (player.position.0 + direction.0 * PLAYER_SPEED).clamp(0.0, state.map_size.0);
                    let new_y = (player.position.1 + direction.1 * PLAYER_SPEED).clamp(0.0, state.map_size.1);
                    (new_x, new_y)
                } else {
                    return Ok(()); // Si le joueur n'existe pas, retournez simplement.
                }
            };
        
            // Ensuite, mettez à jour la position du joueur avec un emprunt mutable.
            if let Some(player) = state.players.get_mut(&addr) {
                player.position = new_position;
            }
        
            // Enfin, diffusez l'état du jeu après avoir libéré l'emprunt mutable.
            broadcast_game_state(&state, &socket).await?;
        }
        
    }
    Ok(())
}

async fn broadcast_game_state(
    state: &GameState,
    socket: &Arc<UdpSocket>,
) -> Result<(), Box<dyn std::error::Error>> {
    let players_state: HashMap<String, (f32, f32)> = state.players
        .iter()
        .map(|(_, player)| (player.name.clone(), player.position))
        .collect();

    let game_state_message = ServerMessage::GameState { players: players_state };
    let serialized = serde_json::to_string(&game_state_message)?;

    for addr in state.players.keys() {
        socket.send_to(serialized.as_bytes(), addr).await?;
    }

    Ok(())
}