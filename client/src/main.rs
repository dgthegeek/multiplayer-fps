mod game_state;
mod player;
mod map;
mod network;
mod messages;
mod ui;
mod camera;
mod input;
mod render;

use bevy::prelude::*;
use game_state::{GameState, AppState};
use network::{setup_network, NetworkReceiver, NetworkSender};
use camera::{MouseSensitivity, PlayerRotation};
use input::CursorState;
use std::io::{self, Write};
use tokio::runtime::Runtime;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;

fn main() -> io::Result<()> {
    println!("Enter server IP:port (e.g., 127.0.0.1:34254): ");
    io::stdout().flush()?;
    let mut server_addr = String::new();
    io::stdin().read_line(&mut server_addr)?;
    let server_addr = server_addr.trim().to_string();

    println!("Enter Name: ");
    io::stdout().flush()?;
    let mut player_name = String::new();
    io::stdin().read_line(&mut player_name)?;
    let player_name = player_name.trim().to_string();

    let rt = Runtime::new().unwrap();
    let (_network_sender, network_receiver, client_sender) = rt.block_on(async {
        setup_network(&server_addr, &player_name).await.unwrap()
    });

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_state::<AppState>()
        .insert_resource(GameState::new(player_name))
        .insert_resource(NetworkReceiver(network_receiver))
        .insert_resource(NetworkSender(client_sender))
        .add_startup_system(render::setup_3d)
        .add_startup_system(ui::setup_ui.after(render::setup_3d))
        .add_system(network::handle_network_messages)
        .add_system(input::player_input)
        .add_system(render::update_player_positions)
        .add_system(render::render_map.in_schedule(OnEnter(AppState::RenderMap)))
        .add_system(ui::update_minimap)  
        .add_system(ui::update_fps_text)  
        .insert_resource(MouseSensitivity(0.005))
        .insert_resource(PlayerRotation::default())
        .add_system(input::player_look)
        .add_startup_system(camera::setup_fps_camera)
        .insert_resource(CursorState { captured: true })
        .add_system(input::toggle_cursor_capture)
        .add_system(ui::game_over_screen.in_schedule(OnEnter(AppState::GameOver)))
        .add_system(ui::display_death_screen)
        .add_system(player::update_bullets)
        .run();

    Ok(())
}