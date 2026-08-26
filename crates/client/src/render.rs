use bevy::prelude::*;

use crate::network::{controls_for_role, GameState, LocalPlayer};

#[derive(Component)]
struct SubmarineMarker;

#[derive(Component)]
struct HudText;

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
            .add_systems(Update, (update_submarine, update_hud));
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

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(16),
                left: px(16),
                right: px(16),
                max_width: px(520),
                padding: UiRect::all(px(14)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.06, 0.09, 0.9)),
            BorderColor::all(Color::srgba(0.2, 0.75, 0.9, 0.75)),
        ))
        .with_child((
            HudText,
            Text::new("SUBMARINE // CONNEXION"),
            TextFont {
                font_size: FontSize::Px(17.0),
                ..default()
            },
            TextColor(Color::srgb(0.82, 0.95, 1.0)),
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

fn update_hud(
    player: Res<LocalPlayer>,
    state: Res<GameState>,
    mut text: Single<&mut Text, With<HudText>>,
) {
    if !player.is_changed() && !state.is_changed() {
        return;
    }

    text.0 = hud_content(&player, &state);
}

fn hud_content(player: &LocalPlayer, state: &GameState) -> String {
    let status = if state.game_started {
        "PARTIE EN COURS"
    } else if player.id.is_some() {
        "EN ATTENTE DE L'EQUIPAGE"
    } else if player.joined {
        "CONNEXION AU LOBBY"
    } else {
        "CONNEXION AU SERVEUR"
    };

    let telemetry = state.submarine.as_ref().map_or_else(
        || "CAP       ---\nVITESSE   ---\nPROFONDEUR ---".to_owned(),
        |submarine| {
            format!(
                "CAP       {:>6.1} deg\nVITESSE   {:>6.1} kn\nPROFONDEUR {:>6.1} m",
                submarine.heading, submarine.speed, submarine.depth
            )
        },
    );

    let error = state
        .last_error
        .as_ref()
        .map(|error| format!("\n\nERREUR SERVEUR\n{error:?}"))
        .unwrap_or_default();

    format!(
        "SUBMARINE // {}\n{}\n\n{}\n\nCOMMANDES\n{}{}",
        role_label(player.role),
        status,
        telemetry,
        controls_for_role(player.role),
        error
    )
}

fn role_label(role: shared::CrewRole) -> &'static str {
    match role {
        shared::CrewRole::Captain => "CAPITAINE",
        shared::CrewRole::Pilot => "PILOTE",
        shared::CrewRole::Sonar => "SONAR",
        shared::CrewRole::Engineer => "INGENIEUR",
        shared::CrewRole::Weapons => "ARMEMENT",
    }
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

    #[test]
    fn hud_reports_role_status_telemetry_and_error() {
        let player = LocalPlayer {
            role: shared::CrewRole::Pilot,
            id: Some(2),
            joined: true,
        };
        let state = GameState {
            submarine: Some(shared::SubmarineState {
                heading: 90.0,
                speed: 12.0,
                depth: 150.0,
                ..default()
            }),
            game_started: true,
            last_error: Some(shared::ProtocolError::CommandNotAllowedForRole),
            ..default()
        };

        let hud = hud_content(&player, &state);

        assert!(hud.contains("PILOTE"));
        assert!(hud.contains("PARTIE EN COURS"));
        assert!(hud.contains("90.0 deg"));
        assert!(hud.contains("CommandNotAllowedForRole"));
    }
}
