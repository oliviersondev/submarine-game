use bevy::prelude::*;

use crate::network::GameState;

#[derive(Component)]
struct SubmarineMarker;

#[derive(Resource, Default)]
struct InterpolationState {
    snapshot_id: u64,
    elapsed: f32,
}

const SNAPSHOT_INTERVAL_SECONDS: f32 = 0.05;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InterpolationState>()
            .add_systems(Startup, setup_scene)
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
    time: Res<Time>,
    state: Res<GameState>,
    mut interpolation: ResMut<InterpolationState>,
    mut query: Query<&mut Transform, With<SubmarineMarker>>,
) {
    let Some(current) = &state.submarine else {
        return;
    };
    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    if interpolation.snapshot_id != state.snapshot_id {
        interpolation.snapshot_id = state.snapshot_id;
        interpolation.elapsed = 0.0;
    }
    interpolation.elapsed += time.delta_secs();

    let alpha = (interpolation.elapsed / SNAPSHOT_INTERVAL_SECONDS).clamp(0.0, 1.0);
    let previous = state.previous_submarine.as_ref().unwrap_or(current);

    transform.translation.x = lerp(previous.x, current.x, alpha);
    transform.translation.y = lerp(previous.y, current.y, alpha);
    transform.rotation =
        Quat::from_rotation_z(-lerp_heading(previous.heading, current.heading, alpha).to_radians());
}

fn lerp(from: f32, to: f32, alpha: f32) -> f32 {
    from + (to - from) * alpha
}

fn lerp_heading(from: f32, to: f32, alpha: f32) -> f32 {
    let delta = (to - from + 180.0).rem_euclid(360.0) - 180.0;
    (from + delta * alpha).rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_position() {
        assert_eq!(lerp(10.0, 20.0, 0.5), 15.0);
    }

    #[test]
    fn interpolates_heading_across_zero_by_shortest_path() {
        assert_eq!(lerp_heading(359.0, 1.0, 0.5), 0.0);
        assert_eq!(lerp_heading(1.0, 359.0, 0.5), 0.0);
    }
}
