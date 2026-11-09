use bevy::prelude::*;
use bevy::window::WindowResolution;

mod button_activation;
mod damage_control_ui;
mod explosion_audio;
mod explosion_ui;
mod network;
mod periscope_effects;
mod periscope_graticule;
mod periscope_ui;
mod render;
mod role;
mod torpedo_ui;
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
        .add_plugins(button_activation::ButtonActivationPlugin)
        .add_plugins(render::RenderPlugin)
        .add_plugins(explosion_ui::ExplosionUiPlugin)
        .add_plugins(explosion_audio::ExplosionAudioPlugin)
        .add_plugins(damage_control_ui::DamageControlUiPlugin)
        .add_plugins(weapons_ui::WeaponsUiPlugin)
        .add_plugins(torpedo_ui::TorpedoUiPlugin)
        .add_plugins(periscope_ui::PeriscopeUiPlugin)
        .run();
}
