use tokio::net::UdpSocket;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use rand::Rng;
use std::io::{self, Write};

const MAP_WIDTH: usize = 20;
const MAP_HEIGHT: usize = 20;
const PLAYER_SPEED: f32 = 0.1;

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Map {
    cells: Vec<Vec<bool>>, // true pour un mur, false pour un espace vide
}

impl Map {
    fn new(difficulty: u8) -> Self {
        let mut rng = rand::thread_rng();
        let mut cells = vec![vec![false; MAP_WIDTH]; MAP_HEIGHT];
        
        // Paramètres de génération basés sur la difficulté
        let (wall_chance, dead_end_chance) = match difficulty {
            1 => (0.1, 0.05),  // Facile: peu de murs et de culs-de-sac
            2 => (0.2, 0.1),   // Moyen: plus de murs et de culs-de-sac
            3 => (0.3, 0.15),  // Difficile: beaucoup de murs et de culs-de-sac
            _ => (0.2, 0.1),   // Par défaut: niveau moyen
        };
        
        // Génération de murs
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                if x == 0 || y == 0 || x == MAP_WIDTH - 1 || y == MAP_HEIGHT - 1 {
                    cells[y][x] = true; // Murs extérieurs
                } else if rng.gen::<f32>() < wall_chance {
                    cells[y][x] = true; // Mur intérieur
                } else if rng.gen::<f32>() < dead_end_chance {
                    // Création d'un cul-de-sac
                    let directions = [(0, 1), (1, 0), (0, -1), (-1, 0)];
                    for _ in 0..3 {
                        let (dx, dy) = directions[rng.gen_range(0..4)];
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && nx < MAP_WIDTH as i32 && ny >= 0 && ny < MAP_HEIGHT as i32 {
                            cells[ny as usize][nx as usize] = true;
                        }
                    }
                }
            }
        }
        
        Map { cells }
    }

    fn is_wall(&self, x: usize, y: usize) -> bool {
        self.cells[y][x]
    }
}

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
    Welcome { map: Map, player_id: String, difficulty: u8 },
    GameState { players: HashMap<String, (f32, f32)> },
}


struct GameState {
    players: HashMap<SocketAddr, Player>,
    map: Map,
    difficulty: u8,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Choose difficulty level (1: Easy, 2: Medium, 3: Hard):");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let difficulty: u8 = input.trim().parse().unwrap_or(2);  // Par défaut à 2 si l'entrée est invalide

    let socket = UdpSocket::bind("0.0.0.0:34254").await?;
    let socket = Arc::new(socket);
    let game_state = Arc::new(Mutex::new(GameState {
        players: HashMap::new(),
        map: Map::new(difficulty),
        difficulty,
    }));


    println!("Server listening on {}", socket.local_addr()?);

    loop {
        let mut buf = vec![0u8; 4096]; 
        let (len, addr) = socket.recv_from(&mut buf).await?;
        let message: ClientMessage = serde_json::from_slice(&buf[..len])?;
        handle_message(message, addr, Arc::clone(&game_state), Arc::clone(&socket)).await?;
    }
}

fn is_valid_move(map: &Map, x: f32, y: f32) -> bool {
    let cell_x = x.floor() as usize;
    let cell_y = y.floor() as usize;
    cell_x < MAP_WIDTH && cell_y < MAP_HEIGHT && !map.is_wall(cell_x, cell_y)
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
            println!("Player connected: {} (IP: {})", name, addr);
            let player = Player {
                name: name.clone(),
                position: (1.0, 1.0), // Position de départ
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
            // Calculez les nouvelles coordonnées du joueur avant d'emprunter `state` de manière mutable.
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

            // Ensuite, mettez à jour la position du joueur avec un emprunt mutable.
            if let Some(new_pos) = new_position {
                if let Some(player) = state.players.get_mut(&addr) {
                    player.position = new_pos;
                }
            }
        }
    }
    broadcast_game_state(&state, &socket).await?;
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
    let game_state_message = ServerMessage::GameState { players: players_state.clone() };
    let serialized = serde_json::to_string(&game_state_message)?;
    
    println!("Broadcasting GameState:");
    for (name, position) in &players_state {
        println!("  Player: {}, Position: {:?}", name, position);
    }
    
    for addr in state.players.keys() {
        socket.send_to(serialized.as_bytes(), addr).await?;
    }
    Ok(())
}