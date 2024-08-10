use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use tokio::time::{interval, Duration};

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
    GameState { players: HashMap<String, (f32, f32)> },
    MapInfo { size: (f32, f32) },
    PlayerMoved { addr: String, position: (f32, f32) }
}


struct GameState {
    players: HashMap<SocketAddr, Player>,
    map_size: (f32, f32),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:34254").await?);
    let game_state = Arc::new(Mutex::new(GameState {
        players: HashMap::new(),
        map_size: MAP_SIZE,
    }));

    println!("Server listening on {}", socket.local_addr()?);

    let state_clone = Arc::clone(&game_state);
    let socket_clone = Arc::clone(&socket);
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_millis(50));
        loop {
            interval.tick().await;
            broadcast_game_state(&state_clone, &socket_clone).await.unwrap();
        }
    });

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
                name,
                position: (0.0, 0.0),
            };
            state.players.insert(addr, player.clone());
            println!("Player joined: {:?} at position {:?}", addr, player.position);
            
            let map_info = ServerMessage::MapInfo { size: state.map_size };
            let serialized = serde_json::to_string(&map_info)?;
            socket.send_to(serialized.as_bytes(), addr).await?;
        }
        ClientMessage::Move { direction } => {
            let map_size = state.map_size;
            if let Some(player) = state.players.get_mut(&addr) {
                let new_x = (player.position.0 + direction.0 * PLAYER_SPEED).clamp(0.0, map_size.0);
                let new_y = (player.position.1 + direction.1 * PLAYER_SPEED).clamp(0.0, map_size.1);
                player.position = (new_x, new_y);
                println!("Player {:?} moved to {:?}", addr, player.position);
        
                // Envoyer la nouvelle position au client
                let response = ServerMessage::PlayerMoved { addr: addr.to_string(), position: player.position };
                let serialized = serde_json::to_string(&response)?;
                socket.send_to(serialized.as_bytes(), addr).await?;
            }
        }
        
    }
    Ok(())
}

async fn broadcast_game_state(game_state: &Arc<Mutex<GameState>>, socket: &Arc<UdpSocket>) -> Result<(), Box<dyn std::error::Error>> {
    let state = game_state.lock().await;
    let players_state: HashMap<String, (f32, f32)> = state.players.iter()
        .map(|(_, player)| (player.name.clone(), player.position))
        .collect();
    
    let message = ServerMessage::GameState { players: players_state };
    let serialized = serde_json::to_string(&message)?;
    
    for addr in state.players.keys() {
        socket.send_to(serialized.as_bytes(), addr).await?;
    }
    
    Ok(())
}