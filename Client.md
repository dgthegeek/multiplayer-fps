# Maze Wars Client

This is the client application for a multiplayer maze game where players navigate through a labyrinth and engage in combat.

## Features

- 3D rendering of the game world
- Real-time multiplayer gameplay
- First-person shooter mechanics
- Minimap for navigation
- FPS counter

## Prerequisites

- Rust  (2021 edition or higher)
- Cargo (Rust package manager)

## Installation

1. Clone this repository:

```
git clone https://learn.zone01dakar.sn/git/dgaye/multiplayer-fps.git
cd multiplayer-fps
```

## Usage

1. Launch the client:
```
cargo run --bin maze_wars_client
```

2. When prompted, enter the server IP and port (e.g., 127.0.0.1:34254).

3. Enter your player name when asked.

## Controls

- WASD: Move
- Mouse: Look around
- Left Mouse Button: Shoot
- Escape: Toggle cursor capture

## Project Structure

- `main.rs`: Application entry point and game loop
- `game_state.rs`: Manages the game state
- `player.rs`: Player-related functionality
- `map.rs`: Map representation and logic
- `network.rs`: Network communication
- `messages.rs`: Defines client-server message structures
- `ui.rs`: User interface elements
- `camera.rs`: Camera management
- `input.rs`: Input handling
- `render.rs`: 3D rendering logic

## Customization

- Adjust `MouseSensitivity` in `main.rs` to change mouse sensitivity
- Modify `PLAYER_SPEED` and `SHOOT_COOLDOWN` in `player.rs` to alter game dynamics
- Change UI elements and styling in `ui.rs`

## Dependencies

- Bevy: Game engine
- Tokio: Asynchronous runtime
- Serde: Serialization and deserialization

## Note

This client is designed to work with the corresponding Maze Wars server. Ensure the server is running and accessible before connecting with the client.
