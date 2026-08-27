use bevy::prelude::*;
use shared::{CrewRole, PlayerCommand, ProtocolError, SubmarineState};

use crate::network::{controls_for_role, CommandQueue, GameState, LocalPlayer};

#[derive(Component)]
struct SubmarineMarker;

#[derive(Component)]
struct MainCamera;

#[derive(Component)]
struct HudText;

#[derive(Component)]
struct HudPanel;

#[derive(Component)]
struct RoleSelector;

#[derive(Component)]
struct SelectorErrorText;

#[derive(Component)]
struct RoleChoice(CrewRole);

#[derive(Component)]
struct PilotPanel;

#[derive(Clone, Copy, Component)]
enum PilotMetric {
    Heading,
    Speed,
    Depth,
}

#[derive(Component)]
struct PilotTelemetry(PilotMetric);

#[derive(Component)]
struct PilotGaugeFill(PilotMetric);

#[derive(Component)]
struct PilotControl {
    metric: PilotMetric,
    direction: f32,
}

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
            .add_systems(
                Update,
                (
                    update_submarine,
                    update_hud,
                    update_panel_visibility,
                    update_selector_error,
                    update_pilot_station,
                    role_button_system,
                    pilot_button_system,
                ),
            );
    }
}

fn setup_scene(mut commands: Commands, player: Res<LocalPlayer>) {
    commands.spawn((Camera2d, MainCamera));

    commands.spawn((
        SubmarineMarker,
        Sprite::from_color(Color::srgb(0.25, 0.75, 1.0), Vec2::new(40.0, 16.0)),
        Transform::default(),
    ));

    commands.spawn((
        HudPanel,
        HudText,
        Text::new("SUBMARINE // CONNEXION"),
        TextFont {
            font_size: FontSize::Px(17.0),
            ..default()
        },
        TextColor(Color::srgb(0.82, 0.95, 1.0)),
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
        GlobalZIndex(20),
        if player.role.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
    ));

    commands.spawn((
        RoleSelector,
        Node {
            position_type: PositionType::Absolute,
            top: px(32),
            left: px(16),
            right: px(16),
            max_width: px(520),
            height: px(390),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.06, 0.09, 0.96)),
        BorderColor::all(Color::srgba(0.2, 0.75, 0.9, 0.85)),
        GlobalZIndex(10),
        selector_visibility(&player),
    ));

    commands.spawn((
        RoleSelector,
        Text::new("CHOISISSEZ VOTRE POSTE"),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextColor(Color::srgb(0.82, 0.95, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: px(52),
            left: px(34),
            ..default()
        },
        GlobalZIndex(20),
        selector_visibility(&player),
    ));

    commands.spawn((
        RoleSelector,
        Text::new("La connexion demarre apres la selection."),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.55, 0.72, 0.78)),
        Node {
            position_type: PositionType::Absolute,
            top: px(84),
            left: px(34),
            ..default()
        },
        GlobalZIndex(20),
        selector_visibility(&player),
    ));

    for (index, (role, label)) in [
        (CrewRole::Captain, "CAPITAINE"),
        (CrewRole::Pilot, "PILOTE"),
        (CrewRole::Sonar, "SONAR"),
        (CrewRole::Engineer, "INGENIEUR"),
        (CrewRole::Weapons, "ARMEMENT"),
    ]
    .into_iter()
    .enumerate()
    {
        commands.spawn((
            RoleSelector,
            RoleChoice(role),
            Button,
            Text::new(format!("{label}  //  {}", role_summary(role))),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(Color::srgb(0.82, 0.95, 1.0)),
            Node {
                position_type: PositionType::Absolute,
                top: px(118.0 + index as f32 * 50.0),
                left: px(34),
                right: px(34),
                max_width: px(484),
                height: px(42),
                padding: UiRect::axes(px(14), px(9)),
                border_radius: BorderRadius::all(px(4)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
            GlobalZIndex(20),
            selector_visibility(&player),
        ));
    }

    commands.spawn((
        RoleSelector,
        SelectorErrorText,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.45, 0.35)),
        Node {
            position_type: PositionType::Absolute,
            top: px(372),
            left: px(34),
            ..default()
        },
        GlobalZIndex(20),
        selector_visibility(&player),
    ));

    spawn_pilot_station(&mut commands, &player);
}

fn spawn_pilot_station(commands: &mut Commands, player: &LocalPlayer) {
    commands
        .spawn((
            PilotPanel,
            Node {
                position_type: PositionType::Absolute,
                left: px(16),
                right: px(16),
                bottom: px(16),
                max_width: px(760),
                min_height: px(214),
                padding: UiRect::all(px(14)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.06, 0.09, 0.94)),
            BorderColor::all(Color::srgba(0.2, 0.75, 0.9, 0.75)),
            GlobalZIndex(20),
            pilot_visibility(player),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("POSTE PILOTE // NAVIGATION"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.82, 0.95, 1.0)),
            ));

            for (metric, label) in [
                (PilotMetric::Heading, "CAP"),
                (PilotMetric::Speed, "VITESSE"),
                (PilotMetric::Depth, "PROFONDEUR"),
            ] {
                panel
                    .spawn(Node {
                        width: percent(100),
                        min_height: px(46),
                        align_items: AlignItems::Center,
                        column_gap: px(6),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.55, 0.72, 0.78)),
                            Node {
                                width: px(66),
                                ..default()
                            },
                        ));
                        spawn_pilot_button(row, metric, -1.0, "-");
                        row.spawn((
                            Node {
                                height: px(8),
                                flex_grow: 1.0,
                                min_width: px(24),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.03, 0.12, 0.16)),
                        ))
                        .with_child((
                            PilotGaugeFill(metric),
                            Node {
                                width: percent(0),
                                height: percent(100),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.75, 0.9)),
                        ));
                        row.spawn((
                            PilotTelemetry(metric),
                            Text::new("---"),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.82, 0.95, 1.0)),
                            Node {
                                width: px(64),
                                ..default()
                            },
                        ));
                        spawn_pilot_button(row, metric, 1.0, "+");
                    });
            }
        });
}

fn spawn_pilot_button(
    parent: &mut ChildSpawnerCommands,
    metric: PilotMetric,
    direction: f32,
    label: &'static str,
) {
    parent
        .spawn((
            PilotControl { metric, direction },
            Button,
            Node {
                width: px(36),
                height: px(38),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
        ))
        .with_child((
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(20.0),
                ..default()
            },
            TextColor(Color::srgb(0.82, 0.95, 1.0)),
        ));
}

fn selector_visibility(player: &LocalPlayer) -> Visibility {
    if player.role.is_none() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn pilot_visibility(player: &LocalPlayer) -> Visibility {
    if player.role == Some(CrewRole::Pilot) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn update_submarine(
    time: Res<Time>,
    state: Res<GameState>,
    mut interpolation: ResMut<InterpolationState>,
    mut submarine: Query<&mut Transform, With<SubmarineMarker>>,
    mut camera: Query<&mut Transform, (With<MainCamera>, Without<SubmarineMarker>)>,
) {
    let Some(current) = &state.submarine else {
        return;
    };
    let Ok(mut transform) = submarine.single_mut() else {
        return;
    };

    if interpolation.snapshot_id != state.snapshot_id {
        interpolation.snapshot_id = state.snapshot_id;
        interpolation.elapsed = 0.0;
    }
    interpolation.elapsed += time.delta_secs();

    let alpha = (interpolation.elapsed / SNAPSHOT_INTERVAL_SECONDS).clamp(0.0, 1.0);
    let previous = state.previous_submarine.as_ref().unwrap_or(current);

    let x = lerp(previous.x, current.x, alpha);
    let y = lerp(previous.y, current.y, alpha);
    transform.translation.x = x;
    transform.translation.y = y;
    transform.translation.z = 0.0;
    transform.rotation =
        Quat::from_rotation_z(-lerp_heading(previous.heading, current.heading, alpha).to_radians());

    if let Ok(mut camera) = camera.single_mut() {
        camera.translation.x = x;
        camera.translation.y = y;
    }
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
    if player.role.is_none() {
        return;
    }

    text.0 = hud_content(&player, &state);
}

fn update_panel_visibility(
    player: Res<LocalPlayer>,
    mut panels: Query<
        (
            &mut Visibility,
            Option<&HudPanel>,
            Option<&RoleSelector>,
            Option<&PilotPanel>,
        ),
        Or<(With<HudPanel>, With<RoleSelector>, With<PilotPanel>)>,
    >,
) {
    if !player.is_changed() {
        return;
    }

    for (mut visibility, hud, selector, pilot) in &mut panels {
        *visibility = if (hud.is_some() && player.role.is_some())
            || (selector.is_some() && player.role.is_none())
            || (pilot.is_some() && player.role == Some(CrewRole::Pilot))
        {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_pilot_station(
    state: Res<GameState>,
    mut telemetry: Query<(&PilotTelemetry, &mut Text)>,
    mut gauges: Query<(&PilotGaugeFill, &mut Node)>,
) {
    if !state.is_changed() {
        return;
    }

    for (telemetry, mut text) in &mut telemetry {
        text.0 = state
            .submarine
            .as_ref()
            .map(|submarine| pilot_metric_text(telemetry.0, submarine))
            .unwrap_or_else(|| "---".to_owned());
    }

    for (gauge, mut node) in &mut gauges {
        node.width = percent(
            state
                .submarine
                .as_ref()
                .map(|submarine| pilot_metric_percent(gauge.0, submarine))
                .unwrap_or(0.0),
        );
    }
}

fn update_selector_error(
    state: Res<GameState>,
    mut text: Single<&mut Text, With<SelectorErrorText>>,
) {
    if !state.is_changed() {
        return;
    }

    text.0 = state
        .last_error
        .as_ref()
        .map(error_label)
        .unwrap_or_default();
}

fn role_button_system(
    mut buttons: Query<
        (&Interaction, &RoleChoice, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut player: ResMut<LocalPlayer>,
    mut state: ResMut<GameState>,
) {
    for (interaction, choice, mut background) in &mut buttons {
        *background = match interaction {
            Interaction::Pressed => {
                player.role = Some(choice.0);
                *state = GameState::default();
                BackgroundColor(Color::srgb(0.08, 0.42, 0.52))
            }
            Interaction::Hovered => BackgroundColor(Color::srgb(0.06, 0.28, 0.36)),
            Interaction::None => BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
        };
    }
}

fn pilot_button_system(
    mut buttons: Query<
        (&Interaction, &PilotControl, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    state: Res<GameState>,
    mut commands: ResMut<CommandQueue>,
) {
    for (interaction, control, mut background) in &mut buttons {
        *background = match interaction {
            Interaction::Pressed => {
                if state.game_started {
                    if let Some(submarine) = &state.submarine {
                        commands.push(pilot_command(control, submarine));
                    }
                }
                BackgroundColor(Color::srgb(0.08, 0.42, 0.52))
            }
            Interaction::Hovered => BackgroundColor(Color::srgb(0.06, 0.28, 0.36)),
            Interaction::None => BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
        };
    }
}

fn pilot_command(control: &PilotControl, submarine: &SubmarineState) -> PlayerCommand {
    match control.metric {
        PilotMetric::Heading => PlayerCommand::SetHeading(
            (submarine.heading + control.direction * 5.0).rem_euclid(360.0),
        ),
        PilotMetric::Speed => {
            PlayerCommand::SetSpeed((submarine.speed + control.direction).clamp(0.0, 20.0))
        }
        PilotMetric::Depth => PlayerCommand::SetDepth(
            (submarine.depth + control.direction * 10.0).clamp(0.0, 1_000.0),
        ),
    }
}

fn pilot_metric_text(metric: PilotMetric, submarine: &SubmarineState) -> String {
    match metric {
        PilotMetric::Heading => format!("{:>5.1} deg", submarine.heading),
        PilotMetric::Speed => format!("{:>5.1} kn", submarine.speed),
        PilotMetric::Depth => format!("{:>5.0} m", submarine.depth),
    }
}

fn pilot_metric_percent(metric: PilotMetric, submarine: &SubmarineState) -> f32 {
    match metric {
        PilotMetric::Heading => submarine.heading.rem_euclid(360.0) / 360.0 * 100.0,
        PilotMetric::Speed => submarine.speed.clamp(0.0, 20.0) / 20.0 * 100.0,
        PilotMetric::Depth => submarine.depth.clamp(0.0, 1_000.0) / 1_000.0 * 100.0,
    }
}

fn hud_content(player: &LocalPlayer, state: &GameState) -> String {
    let Some(role) = player.role else {
        return "SUBMARINE // SELECTION DU POSTE".to_owned();
    };
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

    if role == CrewRole::Pilot {
        format!("SUBMARINE // {}\n{}{}", role_label(role), status, error)
    } else {
        format!(
            "SUBMARINE // {}\n{}\n\n{}\n\nCOMMANDES\n{}{}",
            role_label(role),
            status,
            telemetry,
            controls_for_role(role),
            error
        )
    }
}

fn role_label(role: CrewRole) -> &'static str {
    match role {
        CrewRole::Captain => "CAPITAINE",
        CrewRole::Pilot => "PILOTE",
        CrewRole::Sonar => "SONAR",
        CrewRole::Engineer => "INGENIEUR",
        CrewRole::Weapons => "ARMEMENT",
    }
}

fn role_summary(role: CrewRole) -> &'static str {
    match role {
        CrewRole::Captain => "coordination",
        CrewRole::Pilot => "navigation",
        CrewRole::Sonar => "detection",
        CrewRole::Engineer => "reparations",
        CrewRole::Weapons => "torpilles",
    }
}

fn error_label(error: &ProtocolError) -> String {
    match error {
        ProtocolError::RoleAlreadyTaken(role) => {
            format!("POSTE DEJA PRIS : {}", role_label(*role))
        }
        ProtocolError::CommandNotAllowedForRole => "COMMANDE INTERDITE POUR CE POSTE".to_owned(),
        ProtocolError::GameNotStarted => "LA PARTIE N'A PAS ENCORE DEMARRE".to_owned(),
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
    fn pilot_hud_reports_status_and_leaves_telemetry_to_station() {
        let mut player = LocalPlayer::default();
        player.role = Some(CrewRole::Pilot);
        player.id = Some(2);
        player.joined = true;
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
        assert!(!hud.contains("90.0 deg"));
        assert!(hud.contains("CommandNotAllowedForRole"));
    }

    #[test]
    fn pilot_station_commands_wrap_and_clamp_values() {
        let submarine = SubmarineState {
            heading: 358.0,
            speed: 20.0,
            depth: 0.0,
            ..default()
        };

        assert!(matches!(
            pilot_command(
                &PilotControl {
                    metric: PilotMetric::Heading,
                    direction: 1.0,
                },
                &submarine,
            ),
            PlayerCommand::SetHeading(3.0)
        ));
        assert!(matches!(
            pilot_command(
                &PilotControl {
                    metric: PilotMetric::Speed,
                    direction: 1.0,
                },
                &submarine,
            ),
            PlayerCommand::SetSpeed(20.0)
        ));
        assert!(matches!(
            pilot_command(
                &PilotControl {
                    metric: PilotMetric::Depth,
                    direction: -1.0,
                },
                &submarine,
            ),
            PlayerCommand::SetDepth(0.0)
        ));
    }
}
