use bevy::prelude::*;

mod network;
mod render;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(network::NetworkPlugin)
        .add_plugins(render::RenderPlugin)
        .run();
}
