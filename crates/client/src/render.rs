use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use shared::{
    AlertKind, BallastState, CrewRole, EngineeringMeasurements, LobbyCommand, PilotOrder,
    PlayerCommand, ProtocolError, RoleOccupant, RoomId, SubmarineSnapshot,
};

use crate::network::{controls_for_role, CommandQueue, GameState, LocalPlayer, RoomRequest};

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
struct SelectorCard;

#[derive(Component)]
struct SelectorRoles;

#[derive(Component)]
struct SelectorRoom;

#[derive(Component)]
struct SelectorErrorText;

#[derive(Component)]
struct RoomCodeText;

#[derive(Component)]
struct RoleChoice(CrewRole);

#[derive(Clone, Copy, Component)]
enum SetupAction {
    Create,
    Join,
}

#[derive(Clone, Copy, Component)]
enum CodeKey {
    Digit(char),
    Delete,
}

#[derive(Clone, Copy, Component)]
enum LobbyAction {
    Ready,
    Start,
    OrderPilot,
}

#[derive(Component)]
struct LobbyPanel;

#[derive(Component)]
struct PilotPanel;

#[derive(Component)]
struct EngineerPanel;

#[derive(Component)]
struct EngineerTelemetry;

#[derive(Clone, Copy, Component)]
enum EngineerControl {
    Diesels,
    ElectricMotors,
    Ventilation,
    Charging,
}

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

#[derive(Component)]
struct PilotStatus;

#[derive(Clone, Copy, Component)]
enum PilotAction {
    Ballast(BallastState),
    EmergencySurface,
}

#[derive(Resource, Default)]
struct InterpolationState {
    snapshot_id: u64,
    elapsed: f32,
}

const SNAPSHOT_INTERVAL_SECONDS: f32 = 0.05;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectorLayout {
    Portrait,
    Landscape,
}

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
                    update_panel_visibility.after(crate::network::NetworkReceiveSet),
                    update_selector_error.after(crate::network::NetworkReceiveSet),
                    update_room_code,
                    update_pilot_station,
                    update_lobby_action_layout,
                    update_selector_layout,
                    update_station_layout,
                    role_button_system,
                    setup_button_system,
                    room_code_input,
                    code_key_system,
                    lobby_button_system,
                    pilot_button_system,
                    pilot_action_system,
                    engineer_button_system,
                ),
            );

        #[cfg(target_arch = "wasm32")]
        app.add_systems(PreUpdate, sync_canvas_to_viewport);
    }
}

#[cfg(target_arch = "wasm32")]
fn sync_canvas_to_viewport(mut window: Single<&mut Window, With<PrimaryWindow>>) {
    let Some(browser) = web_sys::window() else {
        return;
    };
    let Some(width) = browser.inner_width().ok().and_then(|value| value.as_f64()) else {
        return;
    };
    let Some(height) = browser.inner_height().ok().and_then(|value| value.as_f64()) else {
        return;
    };
    let (width, height) = (width as f32, height as f32);

    if viewport_size_changed(window.width(), window.height(), width, height) {
        window.resolution.set(width, height);
    }
}

fn viewport_size_changed(current_width: f32, current_height: f32, width: f32, height: f32) -> bool {
    (current_width - width).abs() > 0.5 || (current_height - height).abs() > 0.5
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
        if player.id.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
    ));

    spawn_role_selector(&mut commands, &player);

    spawn_pilot_station(&mut commands, &player);
    spawn_engineer_station(&mut commands, &player);
    spawn_lobby_actions(&mut commands, &player);
}

fn spawn_role_selector(commands: &mut Commands, player: &LocalPlayer) {
    commands
        .spawn((
            RoleSelector,
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                bottom: px(0),
                padding: UiRect::all(px(6)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(10),
            selector_visibility(player),
        ))
        .with_children(|root| {
            root.spawn((
                SelectorCard,
                Node {
                    width: percent(100),
                    max_width: px(520),
                    max_height: percent(100),
                    padding: UiRect::all(px(10)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(8)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(8),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.015, 0.06, 0.09, 0.96)),
                BorderColor::all(Color::srgba(0.2, 0.75, 0.9, 0.85)),
            ))
            .with_children(|card| {
                card.spawn((
                    SelectorRoles,
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        ..default()
                    },
                ))
                .with_children(|roles| {
                    roles.spawn((
                        Text::new("CHOISISSEZ VOTRE POSTE"),
                        TextFont {
                            font_size: FontSize::Px(22.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.82, 0.95, 1.0)),
                    ));
                    roles.spawn((
                        Text::new("Selectionnez un poste."),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.55, 0.72, 0.78)),
                    ));
                    roles.spawn((
                        SelectorErrorText,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.45, 0.35)),
                        Node::default(),
                    ));

                    for (role, label) in [
                        (CrewRole::Captain, "CAPITAINE"),
                        (CrewRole::Pilot, "PILOTE"),
                        (CrewRole::Sonar, "SONAR"),
                        (CrewRole::Engineer, "INGENIEUR"),
                        (CrewRole::Weapons, "ARMEMENT"),
                    ] {
                        roles.spawn((
                            RoleChoice(role),
                            Button,
                            Text::new(format!("{label}  //  {}", role_summary(role))),
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.82, 0.95, 1.0)),
                            Node {
                                width: percent(100),
                                height: px(44),
                                padding: UiRect::horizontal(px(10)),
                                border: UiRect::all(px(1)),
                                border_radius: BorderRadius::all(px(4)),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
                            BorderColor::all(Color::srgba(0.2, 0.75, 0.9, 0.2)),
                        ));
                    }
                });

                card.spawn((
                    SelectorRoom,
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(6),
                        ..default()
                    },
                ))
                .with_children(|room| {
                    room.spawn(Node {
                        width: percent(100),
                        height: px(44),
                        align_items: AlignItems::Center,
                        column_gap: px(6),
                        ..default()
                    })
                    .with_children(|code| {
                        code.spawn((
                            RoomCodeText,
                            Text::new("CODE SALLE : ------"),
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.82, 0.95, 1.0)),
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                        ));
                        code.spawn((
                            CodeKey::Delete,
                            Button,
                            Text::new("EFFACER"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.82, 0.95, 1.0)),
                            Node {
                                width: px(90),
                                height: px(44),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                border_radius: BorderRadius::all(px(4)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
                        ));
                    });

                    room.spawn(Node {
                        width: percent(100),
                        column_gap: px(4),
                        row_gap: px(4),
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    })
                    .with_children(|keypad| {
                        for digit in 0..10 {
                            keypad.spawn((
                                CodeKey::Digit(char::from_digit(digit, 10).unwrap()),
                                Button,
                                Text::new(digit.to_string()),
                                TextFont {
                                    font_size: FontSize::Px(17.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.82, 0.95, 1.0)),
                                Node {
                                    width: percent(18),
                                    min_width: px(44),
                                    height: px(44),
                                    flex_grow: 1.0,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border_radius: BorderRadius::all(px(4)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
                            ));
                        }
                    });

                    room.spawn(Node {
                        width: percent(100),
                        height: px(44),
                        column_gap: px(6),
                        ..default()
                    })
                    .with_children(|actions| {
                        for (action, label) in [
                            (SetupAction::Create, "CREER"),
                            (SetupAction::Join, "REJOINDRE"),
                        ] {
                            actions.spawn((
                                action,
                                Button,
                                Text::new(label),
                                TextFont {
                                    font_size: FontSize::Px(16.0),
                                    ..default()
                                },
                                TextColor(Color::srgb(0.82, 0.95, 1.0)),
                                Node {
                                    height: px(44),
                                    min_width: px(128),
                                    flex_grow: 1.0,
                                    padding: UiRect::horizontal(px(14)),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border_radius: BorderRadius::all(px(4)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
                            ));
                        }
                    });
                });
            });
        });
}

fn spawn_lobby_actions(commands: &mut Commands, player: &LocalPlayer) {
    for (action, label, top) in [
        (LobbyAction::Ready, "PRET", 16.0),
        (LobbyAction::Start, "DEMARRER", 68.0),
        (
            LobbyAction::OrderPilot,
            "ORDRE BOT PILOTE 090 / 8 kn / 50 m",
            16.0,
        ),
    ] {
        commands.spawn((
            LobbyPanel,
            action,
            Button,
            Text::new(label),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::srgb(0.82, 0.95, 1.0)),
            Node {
                position_type: PositionType::Absolute,
                top: px(top),
                right: px(16),
                width: px(220),
                height: px(44),
                padding: UiRect::horizontal(px(12)),
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
            GlobalZIndex(20),
            lobby_action_visibility(player, action),
        ));
    }
}

fn update_lobby_action_layout(
    window: Single<&Window, With<PrimaryWindow>>,
    mut actions: Query<(&LobbyAction, &mut Node)>,
) {
    let landscape = compact_landscape(window.width(), window.height());
    let compact = window.width() < 800.0 && !landscape;

    for (action, mut node) in &mut actions {
        if landscape {
            node.top = Val::Auto;
            node.bottom = px(match action {
                LobbyAction::Ready => 68,
                LobbyAction::Start | LobbyAction::OrderPilot => 16,
            });
            node.left = px(276);
            node.right = px(16);
            node.width = Val::Auto;
            node.max_width = Val::Auto;
        } else if compact {
            node.top = Val::Auto;
            node.bottom = px(match action {
                LobbyAction::Ready => 68,
                LobbyAction::Start | LobbyAction::OrderPilot => 16,
            });
            node.left = px(16);
            node.right = px(16);
            node.width = Val::Auto;
            node.max_width = px(420);
        } else {
            node.top = px(match action {
                LobbyAction::Ready | LobbyAction::OrderPilot => 16,
                LobbyAction::Start => 68,
            });
            node.bottom = Val::Auto;
            node.left = Val::Auto;
            node.right = px(16);
            node.width = px(220);
            node.max_width = Val::Auto;
        }
    }
}

fn update_selector_layout(
    window: Single<&Window, With<PrimaryWindow>>,
    mut nodes: Query<
        (
            &mut Node,
            Option<&SelectorCard>,
            Option<&SelectorRoles>,
            Option<&SelectorRoom>,
        ),
        Or<(With<SelectorCard>, With<SelectorRoles>, With<SelectorRoom>)>,
    >,
) {
    let layout = selector_layout(window.width(), window.height());

    for (mut node, card, roles, room) in &mut nodes {
        if card.is_some() {
            node.max_width = px(match layout {
                SelectorLayout::Portrait => 520,
                SelectorLayout::Landscape => 740,
            });
            node.flex_direction = match layout {
                SelectorLayout::Portrait => FlexDirection::Column,
                SelectorLayout::Landscape => FlexDirection::Row,
            };
            node.row_gap = px(match layout {
                SelectorLayout::Portrait => 8,
                SelectorLayout::Landscape => 0,
            });
            node.column_gap = px(match layout {
                SelectorLayout::Portrait => 0,
                SelectorLayout::Landscape => 12,
            });
        } else if roles.is_some() || room.is_some() {
            match layout {
                SelectorLayout::Portrait => {
                    node.width = percent(100);
                    node.flex_basis = Val::Auto;
                    node.flex_grow = 0.0;
                }
                SelectorLayout::Landscape => {
                    node.width = Val::Auto;
                    node.flex_basis = px(0);
                    node.flex_grow = 1.0;
                }
            }
        }
    }
}

fn selector_layout(width: f32, height: f32) -> SelectorLayout {
    if compact_landscape(width, height) {
        SelectorLayout::Landscape
    } else {
        SelectorLayout::Portrait
    }
}

fn compact_landscape(width: f32, height: f32) -> bool {
    width >= 600.0 && height <= 500.0
}

fn update_station_layout(
    window: Single<&Window, With<PrimaryWindow>>,
    mut nodes: Query<
        (
            &mut Node,
            Option<&HudPanel>,
            Option<&PilotPanel>,
            Option<&EngineerPanel>,
        ),
        Or<(With<HudPanel>, With<PilotPanel>, With<EngineerPanel>)>,
    >,
    mut hud_text: Single<&mut TextFont, With<HudText>>,
) {
    let landscape = compact_landscape(window.width(), window.height());
    hud_text.font_size = FontSize::Px(if landscape { 13.0 } else { 17.0 });

    for (mut node, hud, pilot, engineer) in &mut nodes {
        if hud.is_some() {
            node.left = px(16);
            node.top = px(16);
            if landscape {
                node.right = Val::Auto;
                node.bottom = px(16);
                node.width = px(244);
                node.max_width = px(260);
            } else {
                node.right = px(16);
                node.bottom = Val::Auto;
                node.width = Val::Auto;
                node.max_width = px(520);
            }
        } else if pilot.is_some() || engineer.is_some() {
            node.right = px(16);
            node.bottom = px(16);
            if landscape {
                node.left = px(276);
                node.top = px(16);
                node.width = Val::Auto;
                node.max_width = Val::Auto;
                node.padding = UiRect::all(px(10));
                node.row_gap = px(if pilot.is_some() { 4 } else { 6 });
            } else {
                node.left = px(16);
                node.top = Val::Auto;
                node.width = Val::Auto;
                node.max_width = px(if pilot.is_some() { 760 } else { 620 });
                node.padding = UiRect::all(px(14));
                node.row_gap = px(if pilot.is_some() { 8 } else { 10 });
            }
        }
    }
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
            pilot_visibility(player, false),
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

            panel.spawn((
                PilotStatus,
                Text::new("PLONGEE --- // BALLAST ---"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.72, 0.88, 0.92)),
            ));
            panel
                .spawn(Node {
                    width: percent(100),
                    min_height: px(44),
                    column_gap: px(6),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                })
                .with_children(|row| {
                    for (action, label) in [
                        (PilotAction::Ballast(BallastState::Flood), "REMPLIR"),
                        (PilotAction::Ballast(BallastState::Hold), "TENIR"),
                        (PilotAction::Ballast(BallastState::Blow), "CHASSER"),
                        (PilotAction::EmergencySurface, "SURFACE URGENCE"),
                    ] {
                        row.spawn((
                            action,
                            Button,
                            Text::new(label),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.82, 0.95, 1.0)),
                            Node {
                                min_width: px(74),
                                height: px(44),
                                padding: UiRect::axes(px(10), px(8)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                border_radius: BorderRadius::all(px(4)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
                        ));
                    }
                });
        });
}

fn spawn_engineer_station(commands: &mut Commands, player: &LocalPlayer) {
    commands
        .spawn((
            EngineerPanel,
            Node {
                position_type: PositionType::Absolute,
                left: px(16),
                right: px(16),
                bottom: px(16),
                max_width: px(620),
                min_height: px(230),
                padding: UiRect::all(px(14)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.06, 0.09, 0.94)),
            BorderColor::all(Color::srgba(0.85, 0.65, 0.2, 0.8)),
            GlobalZIndex(20),
            engineer_visibility(player, false),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("POSTE INGENIERIE // ENDURANCE"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.86, 0.5)),
            ));
            panel.spawn((
                EngineerTelemetry,
                Text::new("BATTERIE --- // OXYGENE --- // CHARGE ---"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.82, 0.95, 1.0)),
            ));
            panel
                .spawn(Node {
                    width: percent(100),
                    column_gap: px(8),
                    row_gap: px(8),
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                })
                .with_children(|row| {
                    for (control, label) in [
                        (EngineerControl::Diesels, "DIESELS"),
                        (EngineerControl::ElectricMotors, "MOTEURS ELEC."),
                        (EngineerControl::Ventilation, "VENTILATION"),
                        (EngineerControl::Charging, "RECHARGE"),
                    ] {
                        row.spawn((
                            control,
                            Button,
                            Text::new(label),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.82, 0.95, 1.0)),
                            Node {
                                min_width: px(126),
                                height: px(44),
                                padding: UiRect::axes(px(10), px(8)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                border_radius: BorderRadius::all(px(4)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.12, 0.13, 0.08)),
                        ));
                    }
                });
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
                width: px(44),
                height: px(44),
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
    if player.id.is_none() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn lobby_action_visibility(player: &LocalPlayer, action: LobbyAction) -> Visibility {
    if player.id.is_some()
        && (!matches!(action, LobbyAction::OrderPilot) || player.role == Some(CrewRole::Captain))
    {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn pilot_visibility(player: &LocalPlayer, game_started: bool) -> Visibility {
    if player.role == Some(CrewRole::Pilot) && player.id.is_some() && game_started {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn engineer_visibility(player: &LocalPlayer, game_started: bool) -> Visibility {
    if player.role == Some(CrewRole::Engineer) && player.id.is_some() && game_started {
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

    let x = lerp(previous.common.x, current.common.x, alpha);
    let y = lerp(previous.common.y, current.common.y, alpha);
    transform.translation.x = x;
    transform.translation.y = y;
    transform.translation.z = 0.0;
    transform.rotation = Quat::from_rotation_z(
        -lerp_heading(previous.common.heading, current.common.heading, alpha).to_radians(),
    );

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
    state: Res<GameState>,
    mut panels: Query<
        (
            &mut Visibility,
            Option<&HudPanel>,
            Option<&RoleSelector>,
            Option<&PilotPanel>,
            Option<&EngineerPanel>,
            Option<&LobbyPanel>,
            Option<&LobbyAction>,
        ),
        Or<(
            With<HudPanel>,
            With<RoleSelector>,
            With<PilotPanel>,
            With<EngineerPanel>,
            With<LobbyPanel>,
        )>,
    >,
) {
    if !player.is_changed() && !state.is_changed() {
        return;
    }

    for (mut visibility, hud, selector, pilot, engineer, lobby, lobby_action) in &mut panels {
        let lobby_action_visible = lobby_action.is_some_and(|action| match action {
            LobbyAction::Ready | LobbyAction::Start => !state.game_started,
            LobbyAction::OrderPilot => state.game_started && player.role == Some(CrewRole::Captain),
        });
        *visibility = if (hud.is_some() && player.id.is_some())
            || (selector.is_some() && player.id.is_none())
            || (pilot.is_some()
                && player.role == Some(CrewRole::Pilot)
                && player.id.is_some()
                && state.game_started)
            || (engineer.is_some()
                && player.role == Some(CrewRole::Engineer)
                && player.id.is_some()
                && state.game_started)
            || (lobby.is_some() && player.id.is_some() && lobby_action_visible)
        {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn update_pilot_station(
    state: Res<GameState>,
    mut telemetry: Query<
        (&PilotTelemetry, &mut Text),
        (Without<PilotStatus>, Without<EngineerTelemetry>),
    >,
    mut gauges: Query<(&PilotGaugeFill, &mut Node)>,
    mut status: Single<&mut Text, (With<PilotStatus>, Without<EngineerTelemetry>)>,
    mut engineering: Single<&mut Text, (With<EngineerTelemetry>, Without<PilotStatus>)>,
) {
    if !state.is_changed() {
        return;
    }

    for (telemetry, mut text) in &mut telemetry {
        text.0 = state
            .submarine
            .as_ref()
            .and_then(|submarine| pilot_metric_text(telemetry.0, submarine))
            .unwrap_or_else(|| "---".to_owned());
    }

    for (gauge, mut node) in &mut gauges {
        node.width = percent(
            state
                .submarine
                .as_ref()
                .and_then(|submarine| pilot_metric_percent(gauge.0, submarine))
                .unwrap_or(0.0),
        );
    }

    status.0 = state
        .submarine
        .as_ref()
        .and_then(|submarine| submarine.pilot.as_ref().map(|pilot| (submarine, pilot)))
        .map(|(submarine, pilot)| {
            format!(
                "PLONGEE {:?} // VERT. {:+.1} m/s // VIRAGE {:+.1} deg/s // BALLAST {:?}{}",
                submarine.common.dive_state,
                pilot.vertical_speed,
                pilot.turn_rate,
                pilot.ballast,
                if pilot.emergency_surface {
                    " // URGENCE"
                } else {
                    ""
                }
            )
        })
        .unwrap_or_else(|| "PLONGEE --- // BALLAST ---".to_owned());

    engineering.0 = state
        .submarine
        .as_ref()
        .and_then(|submarine| submarine.engineering.as_ref())
        .map(engineering_text)
        .unwrap_or_else(|| "BATTERIE --- // OXYGENE --- // CHARGE ---".to_owned());
}

fn update_selector_error(
    state: Res<GameState>,
    mut text: Single<&mut Text, With<SelectorErrorText>>,
) {
    text.0 = state
        .last_error
        .as_ref()
        .map(error_label)
        .unwrap_or_default();
}

fn update_room_code(player: Res<LocalPlayer>, mut text: Single<&mut Text, With<RoomCodeText>>) {
    if player.is_changed() {
        text.0 = format!("CODE SALLE : {:_<6}", player.room_code);
    }
}

fn role_button_system(
    mut buttons: Query<(
        &Interaction,
        &RoleChoice,
        &mut BackgroundColor,
        &mut BorderColor,
        &mut Text,
    )>,
    mut player: ResMut<LocalPlayer>,
    mut state: ResMut<GameState>,
) {
    for (interaction, choice, mut background, mut border, mut text) in &mut buttons {
        if *interaction == Interaction::Pressed {
            player.role = Some(choice.0);
            state.last_error = None;
        }

        let selected = player.role == Some(choice.0);
        *background = match (*interaction, selected) {
            (Interaction::Pressed, _) => BackgroundColor(Color::srgb(0.08, 0.42, 0.52)),
            (Interaction::Hovered, _) => BackgroundColor(Color::srgb(0.06, 0.28, 0.36)),
            (Interaction::None, true) => BackgroundColor(Color::srgb(0.05, 0.32, 0.4)),
            (Interaction::None, false) => BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
        };
        *border = BorderColor::all(if selected {
            Color::srgb(0.25, 0.85, 1.0)
        } else {
            Color::srgba(0.2, 0.75, 0.9, 0.2)
        });
        text.0 = if selected {
            format!("> {}  //  {}", role_label(choice.0), role_summary(choice.0))
        } else {
            format!("{}  //  {}", role_label(choice.0), role_summary(choice.0))
        };
    }
}

fn setup_button_system(
    mut buttons: Query<
        (&Interaction, &SetupAction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut player: ResMut<LocalPlayer>,
    mut state: ResMut<GameState>,
) {
    for (interaction, action, mut background) in &mut buttons {
        *background = match interaction {
            Interaction::Pressed => {
                if player.role.is_none() {
                    state.last_error = Some(ProtocolError::CommandNotAllowedForRole);
                } else {
                    state.last_error = None;
                    player.request = match action {
                        SetupAction::Create => Some(RoomRequest::Create),
                        SetupAction::Join if valid_room_code(&player.room_code) => {
                            Some(RoomRequest::Join(RoomId(player.room_code.clone())))
                        }
                        SetupAction::Join => {
                            state.last_error = Some(ProtocolError::InvalidRoomCode);
                            None
                        }
                    };
                }
                BackgroundColor(Color::srgb(0.08, 0.42, 0.52))
            }
            Interaction::Hovered => BackgroundColor(Color::srgb(0.06, 0.28, 0.36)),
            Interaction::None => BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
        };
    }
}

fn room_code_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player: ResMut<LocalPlayer>,
    mut state: ResMut<GameState>,
) {
    if player.request.is_some() {
        return;
    }
    if keyboard.just_pressed(KeyCode::Backspace) {
        player.room_code.pop();
        state.last_error = None;
    }
    if player.room_code.len() >= 6 {
        return;
    }
    for (key, character) in [
        (KeyCode::Digit0, '0'),
        (KeyCode::Digit1, '1'),
        (KeyCode::Digit2, '2'),
        (KeyCode::Digit3, '3'),
        (KeyCode::Digit4, '4'),
        (KeyCode::Digit5, '5'),
        (KeyCode::Digit6, '6'),
        (KeyCode::Digit7, '7'),
        (KeyCode::Digit8, '8'),
        (KeyCode::Digit9, '9'),
    ] {
        if keyboard.just_pressed(key) {
            player.room_code.push(character);
            state.last_error = None;
            break;
        }
    }
}

fn code_key_system(
    mut buttons: Query<
        (&Interaction, &CodeKey, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut player: ResMut<LocalPlayer>,
    mut state: ResMut<GameState>,
) {
    if player.request.is_some() {
        return;
    }
    for (interaction, key, mut background) in &mut buttons {
        *background = match interaction {
            Interaction::Pressed => {
                match key {
                    CodeKey::Digit(digit) if player.room_code.len() < 6 => {
                        player.room_code.push(*digit);
                        state.last_error = None;
                    }
                    CodeKey::Delete => {
                        player.room_code.pop();
                        state.last_error = None;
                    }
                    CodeKey::Digit(_) => {}
                }
                BackgroundColor(Color::srgb(0.08, 0.42, 0.52))
            }
            Interaction::Hovered => BackgroundColor(Color::srgb(0.06, 0.28, 0.36)),
            Interaction::None => BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
        };
    }
}

fn lobby_button_system(
    mut buttons: Query<
        (&Interaction, &LobbyAction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    state: Res<GameState>,
    mut commands: ResMut<CommandQueue>,
) {
    for (interaction, action, mut background) in &mut buttons {
        *background = match interaction {
            Interaction::Pressed => {
                match action {
                    LobbyAction::Ready => commands.lobby(LobbyCommand::SetReady { ready: true }),
                    LobbyAction::Start => commands.lobby(LobbyCommand::StartMission),
                    LobbyAction::OrderPilot if state.game_started => {
                        commands.order_pilot(PilotOrder {
                            heading: 90.0,
                            speed: 8.0,
                            depth: 50.0,
                        });
                    }
                    LobbyAction::OrderPilot => {}
                }
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
                        let command = pilot_command(control, submarine, &commands);
                        commands.push(command);
                    }
                }
                BackgroundColor(Color::srgb(0.08, 0.42, 0.52))
            }
            Interaction::Hovered => BackgroundColor(Color::srgb(0.06, 0.28, 0.36)),
            Interaction::None => BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
        };
    }
}

fn pilot_action_system(
    mut buttons: Query<
        (&Interaction, &PilotAction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    state: Res<GameState>,
    mut commands: ResMut<CommandQueue>,
) {
    for (interaction, action, mut background) in &mut buttons {
        *background = match interaction {
            Interaction::Pressed => {
                if state.game_started {
                    commands.push(match action {
                        PilotAction::Ballast(ballast) => PlayerCommand::SetBallast(*ballast),
                        PilotAction::EmergencySurface => PlayerCommand::EmergencySurface,
                    });
                }
                BackgroundColor(Color::srgb(0.08, 0.42, 0.52))
            }
            Interaction::Hovered => BackgroundColor(Color::srgb(0.06, 0.28, 0.36)),
            Interaction::None => BackgroundColor(Color::srgb(0.04, 0.16, 0.21)),
        };
    }
}

fn engineer_button_system(
    mut buttons: Query<(&Interaction, &EngineerControl, &mut BackgroundColor), With<Button>>,
    state: Res<GameState>,
    mut commands: ResMut<CommandQueue>,
) {
    let engineering = state
        .submarine
        .as_ref()
        .and_then(|submarine| submarine.engineering.as_ref());
    for (interaction, control, mut background) in &mut buttons {
        let active = engineering
            .is_some_and(|engineering| engineer_control_active(*control, engineering, &commands));
        *background = match interaction {
            Interaction::Pressed => {
                if state.game_started {
                    if let Some(engineering) = engineering {
                        let command = engineer_command(*control, engineering, &commands);
                        commands.push(command);
                    }
                }
                BackgroundColor(Color::srgb(0.35, 0.3, 0.08))
            }
            Interaction::Hovered => BackgroundColor(Color::srgb(0.28, 0.25, 0.08)),
            Interaction::None if active => BackgroundColor(Color::srgb(0.32, 0.27, 0.06)),
            Interaction::None => BackgroundColor(Color::srgb(0.12, 0.13, 0.08)),
        };
    }
}

fn engineer_control_active(
    control: EngineerControl,
    state: &EngineeringMeasurements,
    commands: &CommandQueue,
) -> bool {
    match control {
        EngineerControl::Diesels => commands.pending_diesels(state.propulsion.diesels_on),
        EngineerControl::ElectricMotors => {
            commands.pending_electric_motors(state.propulsion.electric_motors_on)
        }
        EngineerControl::Ventilation => {
            commands.pending_ventilation(state.propulsion.ventilation_on)
        }
        EngineerControl::Charging => commands.pending_charging(state.propulsion.charging),
    }
}

fn engineer_command(
    control: EngineerControl,
    state: &EngineeringMeasurements,
    commands: &CommandQueue,
) -> PlayerCommand {
    match control {
        EngineerControl::Diesels => {
            PlayerCommand::SetDiesels(!commands.pending_diesels(state.propulsion.diesels_on))
        }
        EngineerControl::ElectricMotors => PlayerCommand::SetElectricMotors(
            !commands.pending_electric_motors(state.propulsion.electric_motors_on),
        ),
        EngineerControl::Ventilation => PlayerCommand::SetVentilation(
            !commands.pending_ventilation(state.propulsion.ventilation_on),
        ),
        EngineerControl::Charging => {
            PlayerCommand::SetBatteryCharging(!commands.pending_charging(state.propulsion.charging))
        }
    }
}

fn pilot_command(
    control: &PilotControl,
    submarine: &SubmarineSnapshot,
    commands: &CommandQueue,
) -> PlayerCommand {
    let pilot = submarine.pilot.as_ref().expect("pilot projection");
    match control.metric {
        PilotMetric::Heading => PlayerCommand::SetHeading(
            (commands.pending_heading(pilot.ordered_heading) + control.direction * 5.0)
                .rem_euclid(360.0),
        ),
        PilotMetric::Speed => PlayerCommand::SetSpeed(
            (commands.pending_speed(pilot.ordered_speed) + control.direction).clamp(0.0, 18.0),
        ),
        PilotMetric::Depth => PlayerCommand::SetDepth(
            (commands.pending_depth(pilot.ordered_depth) + control.direction * 10.0)
                .clamp(0.0, pilot.max_depth),
        ),
    }
}

fn pilot_metric_text(metric: PilotMetric, submarine: &SubmarineSnapshot) -> Option<String> {
    let pilot = submarine.pilot.as_ref()?;
    Some(match metric {
        PilotMetric::Heading => format!(
            "{:>3.0}/{:>3.0} deg",
            submarine.common.heading, pilot.ordered_heading
        ),
        PilotMetric::Speed => format!(
            "{:>3.1}/{:>3.1} kn",
            submarine.common.speed, pilot.ordered_speed
        ),
        PilotMetric::Depth => format!(
            "{:>3.0}/{:>3.0} m",
            submarine.common.depth, pilot.ordered_depth
        ),
    })
}

fn pilot_metric_percent(metric: PilotMetric, submarine: &SubmarineSnapshot) -> Option<f32> {
    let pilot = submarine.pilot.as_ref()?;
    Some(match metric {
        PilotMetric::Heading => submarine.common.heading.rem_euclid(360.0) / 360.0 * 100.0,
        PilotMetric::Speed => {
            submarine.common.speed.clamp(0.0, pilot.max_speed) / pilot.max_speed * 100.0
        }
        PilotMetric::Depth => {
            submarine.common.depth.clamp(0.0, pilot.max_depth) / pilot.max_depth * 100.0
        }
    })
}

fn engineering_text(engineering: &EngineeringMeasurements) -> String {
    format!(
        "BATTERIE {:>5.1}% // OXYGENE {:>5.1}% // CHARGE {:>5.2}%/s\nPRISE D'AIR {} // DIESELS {} // ELEC. {} // VENT. {} // RECHARGE {}",
        engineering.battery,
        engineering.oxygen,
        engineering.electrical_load,
        on_off(engineering.air_intake_available),
        on_off(engineering.propulsion.diesels_on),
        on_off(engineering.propulsion.electric_motors_on),
        on_off(engineering.propulsion.ventilation_on),
        on_off(engineering.propulsion.charging),
    )
}

fn on_off(active: bool) -> &'static str {
    if active {
        "ON"
    } else {
        "OFF"
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

    let lobby = state.lobby.as_ref().map_or_else(String::new, |lobby| {
        if state.game_started {
            return format!("\nSALLE {} // TICK {}", lobby.room_id.0, state.server_tick);
        }
        let slots = lobby
            .slots
            .iter()
            .map(|slot| match slot.occupant {
                RoleOccupant::Human { ready, .. } => format!(
                    "{} : HUMAIN {}",
                    role_label(slot.role),
                    if ready { "PRET" } else { "ATTENTE" }
                ),
                RoleOccupant::Bot => format!("{} : BOT", role_label(slot.role)),
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "\nSALLE {} // TICK {}\n{}",
            lobby.room_id.0, state.server_tick, slots
        )
    });

    let telemetry = state.submarine.as_ref().map_or_else(
        || "CAP       ---\nVITESSE   ---\nPROFONDEUR ---".to_owned(),
        |submarine| {
            format!(
                "CAP       {:>6.1} deg\nVITESSE   {:>6.1} kn\nPROFONDEUR {:>6.1} m",
                submarine.common.heading, submarine.common.speed, submarine.common.depth
            )
        },
    );

    let alerts = state.submarine.as_ref().map_or_else(
        || "\nBRUIT     ---\nALERTES   ---".to_owned(),
        |submarine| {
            if submarine.common.alerts.is_empty() {
                format!(
                    "\nBRUIT     {:?}\nALERTES   AUCUNE",
                    submarine.common.acoustic_level
                )
            } else {
                format!(
                    "\nBRUIT     {:?}\nALERTES   {}",
                    submarine.common.acoustic_level,
                    submarine
                        .common
                        .alerts
                        .iter()
                        .map(alert_label)
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            }
        },
    );

    let error = state
        .last_error
        .as_ref()
        .map(|error| format!("\n\nERREUR SERVEUR\n{}", error_label(error)))
        .unwrap_or_default();

    if matches!(role, CrewRole::Pilot | CrewRole::Engineer) {
        format!(
            "SUBMARINE // {}\n{}{}{}{}",
            role_label(role),
            status,
            lobby,
            alerts,
            error
        )
    } else {
        format!(
            "SUBMARINE // {}\n{}{}{}\n\n{}\n\nCOMMANDES\n{}{}",
            role_label(role),
            status,
            lobby,
            alerts,
            telemetry,
            controls_for_role(role),
            error
        )
    }
}

fn alert_label(alert: &AlertKind) -> &'static str {
    match alert {
        AlertKind::BatteryLow => "BATTERIE BASSE",
        AlertKind::AirCritical => "AIR CRITIQUE",
        AlertKind::Cavitation => "CAVITATION",
        AlertKind::CriticalDepth => "PROFONDEUR CRITIQUE",
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
        ProtocolError::IncompatibleVersion { .. } => "VERSION DE PROTOCOLE INCOMPATIBLE".to_owned(),
        ProtocolError::RoomNotFound => "SALLE INTROUVABLE".to_owned(),
        ProtocolError::RoomAlreadyStarted => "MISSION DEJA DEMARREE".to_owned(),
        ProtocolError::PilotControlledByHuman => {
            "ORDRE BOT BLOQUE : LE PILOTE EST HUMAIN".to_owned()
        }
        ProtocolError::InvalidRoomCode => "CODE SALLE INVALIDE : 6 CHIFFRES REQUIS".to_owned(),
        ProtocolError::ConnectionFailed => "CONNEXION AU SERVEUR IMPOSSIBLE".to_owned(),
    }
}

fn valid_room_code(code: &str) -> bool {
    code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit())
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
        player.id = Some(shared::PlayerId(2));
        player.joined = true;
        let state = GameState {
            submarine: Some(pilot_snapshot(90.0, 12.0, 150.0)),
            game_started: true,
            last_error: Some(shared::ProtocolError::CommandNotAllowedForRole),
            ..default()
        };

        let hud = hud_content(&player, &state);

        assert!(hud.contains("PILOTE"));
        assert!(hud.contains("PARTIE EN COURS"));
        assert!(hud.contains("BRUIT     Low"));
        assert!(!hud.contains("90.0 deg"));
        assert!(hud.contains("COMMANDE INTERDITE POUR CE POSTE"));
    }

    #[test]
    fn engineer_hud_leaves_detailed_controls_to_station_panel() {
        let mut player = LocalPlayer::default();
        player.role = Some(CrewRole::Engineer);
        player.id = Some(shared::PlayerId(3));
        player.joined = true;
        let state = GameState {
            submarine: Some(pilot_snapshot(0.0, 0.0, 0.0)),
            game_started: true,
            ..default()
        };

        let hud = hud_content(&player, &state);

        assert!(hud.contains("INGENIEUR"));
        assert!(hud.contains("BRUIT"));
        assert!(!hud.contains("COMMANDES"));
    }

    #[test]
    fn pilot_station_commands_wrap_and_clamp_values() {
        let submarine = pilot_snapshot(358.0, 18.0, 0.0);
        let commands = CommandQueue::default();

        assert!(matches!(
            pilot_command(
                &PilotControl {
                    metric: PilotMetric::Heading,
                    direction: 1.0,
                },
                &submarine,
                &commands,
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
                &commands,
            ),
            PlayerCommand::SetSpeed(18.0)
        ));
        assert!(matches!(
            pilot_command(
                &PilotControl {
                    metric: PilotMetric::Depth,
                    direction: -1.0,
                },
                &submarine,
                &commands,
            ),
            PlayerCommand::SetDepth(0.0)
        ));
    }

    #[test]
    fn rapid_pilot_commands_accumulate_between_snapshots() {
        let submarine = pilot_snapshot(0.0, 10.0, 0.0);
        let control = PilotControl {
            metric: PilotMetric::Speed,
            direction: 1.0,
        };
        let mut commands = CommandQueue::default();
        let first = pilot_command(&control, &submarine, &commands);
        commands.push(first);

        assert_eq!(
            pilot_command(&control, &submarine, &commands),
            PlayerCommand::SetSpeed(12.0)
        );
    }

    #[test]
    fn non_pilot_projection_is_safe_for_hidden_pilot_widgets() {
        let mut snapshot = pilot_snapshot(0.0, 0.0, 0.0);
        snapshot.pilot = None;

        assert_eq!(pilot_metric_text(PilotMetric::Heading, &snapshot), None);
        assert_eq!(pilot_metric_percent(PilotMetric::Heading, &snapshot), None);
    }

    #[test]
    fn room_code_requires_exactly_six_digits() {
        assert!(valid_room_code("000001"));
        assert!(!valid_room_code("11"));
        assert!(!valid_room_code("00000A"));
        assert!(!valid_room_code("0000001"));
    }

    #[test]
    fn selector_uses_expected_layout_at_target_dimensions() {
        assert_eq!(selector_layout(740.0, 360.0), SelectorLayout::Landscape);
        assert_eq!(selector_layout(640.0, 360.0), SelectorLayout::Landscape);
        assert_eq!(selector_layout(360.0, 740.0), SelectorLayout::Portrait);
        assert_eq!(selector_layout(360.0, 640.0), SelectorLayout::Portrait);
    }

    #[test]
    fn compact_landscape_breakpoint_requires_both_dimensions() {
        assert!(compact_landscape(600.0, 500.0));
        assert!(!compact_landscape(599.0, 500.0));
        assert!(!compact_landscape(600.0, 501.0));
    }

    #[test]
    fn viewport_resize_ignores_subpixel_noise() {
        assert!(!viewport_size_changed(360.0, 640.0, 360.4, 639.6));
        assert!(viewport_size_changed(360.0, 640.0, 361.0, 640.0));
        assert!(viewport_size_changed(360.0, 640.0, 360.0, 641.0));
    }

    #[test]
    fn setup_errors_have_readable_labels() {
        assert_eq!(
            error_label(&ProtocolError::RoomNotFound),
            "SALLE INTROUVABLE"
        );
        assert_eq!(
            error_label(&ProtocolError::RoleAlreadyTaken(CrewRole::Pilot)),
            "POSTE DEJA PRIS : PILOTE"
        );
        assert_eq!(
            error_label(&ProtocolError::ConnectionFailed),
            "CONNEXION AU SERVEUR IMPOSSIBLE"
        );
    }

    #[test]
    fn station_panels_stay_hidden_until_session_is_joined() {
        let mut player = LocalPlayer::default();
        player.role = Some(CrewRole::Pilot);
        player.request = Some(RoomRequest::Join(RoomId("999999".to_owned())));

        assert_eq!(selector_visibility(&player), Visibility::Visible);
        assert_eq!(pilot_visibility(&player, false), Visibility::Hidden);

        player.id = Some(shared::PlayerId(1));

        assert_eq!(selector_visibility(&player), Visibility::Hidden);
        assert_eq!(pilot_visibility(&player, false), Visibility::Hidden);
        assert_eq!(pilot_visibility(&player, true), Visibility::Visible);
    }

    fn pilot_snapshot(heading: f32, speed: f32, depth: f32) -> SubmarineSnapshot {
        SubmarineSnapshot {
            common: shared::CommonMeasurements {
                x: 0.0,
                y: 0.0,
                heading,
                speed,
                depth,
                dive_state: shared::DiveState::Surface,
                acoustic_level: shared::AcousticLevel::Low,
                alerts: vec![],
            },
            pilot: Some(shared::PilotMeasurements {
                ordered_heading: heading,
                ordered_speed: speed,
                ordered_depth: depth,
                turn_rate: 0.0,
                vertical_speed: 0.0,
                ballast: BallastState::Hold,
                emergency_surface: false,
                max_speed: 18.0,
                max_depth: 250.0,
            }),
            engineering: None,
        }
    }
}
