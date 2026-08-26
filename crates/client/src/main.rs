use bevy::prelude::*;

mod network;
mod render;
mod role;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(network::NetworkPlugin)
        .add_plugins(render::RenderPlugin)
        .run();
}
