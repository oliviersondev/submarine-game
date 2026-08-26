use bevy::prelude::*;

use crate::network::GameState;

#[derive(Component)]
struct SubmarineMarker;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_scene)
            .add_systems(Update, update_submarine);
    }
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    commands.spawn((
        SubmarineMarker,
        Mesh2d(meshes.add(Rectangle::new(40.0, 16.0))),
        MeshMaterial2d(materials.add(Color::srgb(0.25, 0.75, 1.0))),
        Transform::default(),
    ));
}

fn update_submarine(
    state: Res<GameState>,
    mut query: Query<&mut Transform, With<SubmarineMarker>>,
) {
    let Some(sub) = &state.submarine else { return };
    let Ok(mut transform) = query.single_mut() else { return };

    transform.translation.x = sub.x;
    transform.translation.y = sub.y;
    transform.rotation = Quat::from_rotation_z(-sub.heading.to_radians());
}
