use bevy::prelude::*;

mod ai;
mod components;
mod rendering;
mod game_plugin;

use game_plugin::GamePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Shed".to_string(),
                resolution: (1440.0, 900.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GamePlugin)
        .run();
}
