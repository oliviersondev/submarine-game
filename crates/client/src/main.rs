use bevy::prelude::*;
use bevy::window::WindowResolution;

mod explosion_ui;
mod network;
mod render;
mod role;
mod weapons_ui;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Submarine Game".into(),
                canvas: Some("#game-canvas".into()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(network::NetworkPlugin)
        .add_plugins(render::RenderPlugin)
        .add_plugins(explosion_ui::ExplosionUiPlugin)
        .add_plugins(weapons_ui::WeaponsUiPlugin)
        .run();
}
