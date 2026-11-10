use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use shared::{
    CrewRole, PeriscopeBlock, PilotOrder, PlayerCommand, SilhouetteKind, VisualContactId,
};

use crate::{
    button_activation::{self, ButtonActivated},
    network::{CommandQueue, GameState, LocalPlayer},
};

#[derive(Resource)]
pub(crate) struct PeriscopeUi {
    pub(crate) open: bool,
    selected_observation: Option<u64>,
    selected_contact: Option<VisualContactId>,
    pending_contact: Option<u64>,
    raise_attempted: bool,
    drag_origin: f32,
    pilot: PilotOrder,
    distance: f32,
    target_heading: f32,
    target_speed: f32,
}
impl Default for PeriscopeUi {
    fn default() -> Self {
        Self {
            open: false,
            selected_observation: None,
            selected_contact: None,
            pending_contact: None,
            raise_attempted: false,
            drag_origin: 0.0,
            distance: 1000.0,
            target_heading: 90.0,
            target_speed: 8.0,
            pilot: PilotOrder {
                heading: 0.0,
                speed: 2.0,
                depth: 12.0,
            },
        }
    }
}

#[derive(Component, Clone, Copy)]
enum Action {
    Toggle,
    Raise,
    Left,
    Right,
    Create,
    NextContact,
    Share,
    PilotHeading(f32),
    PilotSpeed(f32),
    PilotDepth(f32),
    OrderPilot,
    Distance(f32),
    TargetHeading(f32),
    TargetSpeed(f32),
    ApplyEstimate,
    OrderVisual,
}

const CLICK_FLASH_SECONDS: f64 = 0.15;

#[derive(Component, Default)]
struct ButtonFeedback {
    flash_until: f64,
}

#[derive(Debug, PartialEq, Eq)]
enum ButtonAppearance {
    Idle,
    Hovered,
    Pressed,
    Flash,
    Disabled,
}
#[derive(Component)]
struct Panel;
#[derive(Component)]
struct CommandPanels;
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
enum StationSection {
    Periscope,
    Pilot,
    Weapons,
}
#[derive(Component)]
struct Toggle;
#[derive(Component)]
struct ToggleLabel;
#[derive(Component)]
struct CommandButtonLabel;
#[derive(Component)]
struct Status;
#[derive(Component)]
struct ContactStatus;
#[derive(Component)]
struct PilotStatus;
#[derive(Component)]
struct EstimateStatus;
#[derive(Component)]
struct AppliedEstimateStatus;
#[derive(Component)]
pub(crate) struct View;
#[derive(Component)]
struct ClosedViewMessage;
#[derive(Component)]
struct ClosedViewTitle;
#[derive(Component)]
struct ClosedViewDetail;
#[derive(Component)]
struct SharedContacts;
#[derive(Component)]
struct SceneGeometry;
#[derive(Component)]
struct Slot(usize);
#[derive(Component)]
struct Hull(usize);
#[derive(Component)]
struct Funnel(usize, usize);

pub struct PeriscopeUiPlugin;
impl Plugin for PeriscopeUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::periscope_graticule::PeriscopeGraticulePlugin)
            .add_plugins(crate::periscope_effects::PeriscopeEffectsPlugin)
            .init_resource::<PeriscopeUi>()
            .add_systems(Startup, spawn)
            .add_systems(
                Update,
                (
                    buttons,
                    update_toggle_label,
                    update_command_layout,
                    scroll_panel,
                    update,
                    update_shared_contacts,
                    update_estimate_status,
                    update_button_feedback,
                )
                    .chain()
                    .after(crate::network::NetworkReceiveSet),
            );
    }
}

fn label(parent: &mut ChildSpawnerCommands, text: &str) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Pickable::IGNORE,
        Node {
            flex_shrink: 0.0,
            min_width: px(0),
            max_width: percent(100),
            ..default()
        },
    ));
}

fn button(parent: &mut ChildSpawnerCommands, action: Action, text: &str) {
    let (fill, outline) = action_section(action).colors(&ButtonAppearance::Idle);
    let mut entity = parent.spawn((
        Button,
        action,
        ButtonFeedback::default(),
        BorderColor::all(outline),
        Node {
            min_height: px(44),
            min_width: px(44),
            flex_grow: 1.0,
            flex_basis: px(0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::all(px(4)),
            border: UiRect::all(px(1)),
            ..default()
        },
        BackgroundColor(fill),
    ));
    button_activation::enable(&mut entity);
    entity.with_children(|p| {
        p.spawn((
            CommandButtonLabel,
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextLayout::justify(Justify::Center),
            TextColor(Color::WHITE),
            Pickable::IGNORE,
            // An auto width bounded only by 100% can collapse during flex-basis: 0
            // measurement. Reserve the button's content width, including for wrapped labels.
            Node {
                width: percent(100),
                min_width: px(0),
                ..default()
            },
        ));
    });
}

fn action_section(action: Action) -> StationSection {
    match action {
        Action::PilotHeading(_)
        | Action::PilotSpeed(_)
        | Action::PilotDepth(_)
        | Action::OrderPilot => StationSection::Pilot,
        Action::Distance(_)
        | Action::TargetHeading(_)
        | Action::TargetSpeed(_)
        | Action::ApplyEstimate
        | Action::OrderVisual => StationSection::Weapons,
        _ => StationSection::Periscope,
    }
}

impl StationSection {
    fn colors(self, appearance: &ButtonAppearance) -> (Color, Color) {
        if *appearance == ButtonAppearance::Disabled {
            return (
                Color::srgb(0.035, 0.075, 0.085),
                Color::srgb(0.10, 0.18, 0.20),
            );
        }
        let strength = match appearance {
            ButtonAppearance::Idle => 0.0,
            ButtonAppearance::Hovered => 0.08,
            ButtonAppearance::Pressed => 0.16,
            ButtonAppearance::Flash => 0.22,
            ButtonAppearance::Disabled => unreachable!(),
        };
        match self {
            Self::Periscope => (
                Color::srgb(0.04, 0.20 + strength, 0.25 + strength),
                Color::srgb(0.18, 0.60 + strength, 0.65 + strength),
            ),
            Self::Pilot => (
                Color::srgb(0.07, 0.13 + strength, 0.27 + strength),
                Color::srgb(0.35, 0.52 + strength, 0.75 + strength),
            ),
            Self::Weapons => (
                Color::srgb(0.25 + strength, 0.16 + strength, 0.055),
                Color::srgb(0.75 + strength, 0.53 + strength, 0.20),
            ),
        }
    }
}

fn section(
    parent: &mut ChildSpawnerCommands,
    group: StationSection,
    title: &str,
    subtitle: &str,
    contents: impl FnOnce(&mut ChildSpawnerCommands),
) {
    let (_, accent) = group.colors(&ButtonAppearance::Idle);
    parent
        .spawn((
            group,
            Node {
                width: percent(100),
                min_width: px(0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                padding: UiRect::all(px(10)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BorderColor::all(accent),
            BackgroundColor(match group {
                StationSection::Periscope => Color::srgb(0.02, 0.085, 0.10),
                StationSection::Pilot => Color::srgb(0.025, 0.045, 0.095),
                StationSection::Weapons => Color::srgb(0.09, 0.06, 0.025),
            }),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(accent),
                Pickable::IGNORE,
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            label(card, subtitle);
            contents(card);
        });
}

fn action_row(parent: &mut ChildSpawnerCommands, actions: &[(Action, &str)]) {
    parent
        .spawn(Node {
            width: percent(100),
            column_gap: px(6),
            flex_shrink: 0.0,
            ..default()
        })
        .with_children(|row| {
            for &(action, text) in actions {
                button(row, action, text);
            }
        });
}

fn wide_command_layout(width: f32) -> bool {
    width >= 960.0
}

fn update_command_layout(
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
    mut nodes: Query<
        (&mut Node, Option<&StationSection>, Has<CommandPanels>),
        Or<(With<StationSection>, With<CommandPanels>)>,
    >,
) {
    let wide = wide_command_layout(window.width());
    for (mut node, section, panels) in &mut nodes {
        if panels {
            node.flex_direction = if wide {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            };
        } else if section != Some(&StationSection::Periscope) {
            node.width = if wide { Val::Auto } else { percent(100) };
            node.flex_basis = if wide { px(0) } else { Val::Auto };
            node.flex_grow = if wide { 1.0 } else { 0.0 };
        }
    }
}

fn spawn(mut commands: Commands) {
    let mut toggle = commands.spawn((
        Toggle,
        Button,
        Action::Toggle,
        ButtonFeedback::default(),
        BorderColor::all(Color::srgb(0.15, 0.38, 0.43)),
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            right: px(18),
            top: px(52),
            width: px(220),
            min_width: px(180),
            max_width: percent(90),
            min_height: px(44),
            padding: UiRect::all(px(8)),
            border: UiRect::all(px(1)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        GlobalZIndex(71),
        BackgroundColor(Color::srgb(0.03, 0.24, 0.28)),
    ));
    button_activation::enable(&mut toggle);
    toggle.with_children(|p| {
        p.spawn((
            ToggleLabel,
            Text::new(toggle_label(false)),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextLayout::justify(Justify::Center),
            TextColor(Color::WHITE),
            Pickable::IGNORE,
            Node {
                width: percent(100),
                min_width: px(0),
                max_width: percent(100),
                ..default()
            },
        ));
    });
    commands
        .spawn((
            Panel,
            RelativeCursorPosition::default(),
            ScrollPosition::default(),
            Visibility::Hidden,
            Node {
                position_type: PositionType::Absolute,
                left: px(8),
                right: px(8),
                top: px(104),
                bottom: px(8),
                padding: UiRect::all(px(8)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            GlobalZIndex(70),
            BackgroundColor(Color::srgb(0.015, 0.06, 0.075)),
        ))
        .observe(
            |drag: On<Pointer<Drag>>,
             scale: Res<UiScale>,
             mut panels: Query<(&ComputedNode, &mut ScrollPosition), With<Panel>>| {
                // Vertical scrolling remains available on compact landscape screens.
                if let Ok((node, mut scroll)) = panels.single_mut() {
                    let maximum =
                        (node.content_size.y - node.size.y).max(0.0) * node.inverse_scale_factor;
                    scroll.0.y = (scroll.0.y - drag.delta.y / scale.0).clamp(0.0, maximum);
                }
            },
        )
        .with_children(|root| {
            root.spawn((
                Status,
                Text::new("PERISCOPE"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            section(root, StationSection::Periscope, "PERISCOPE // OBSERVER", "Orienter la vue, reperer et partager les contacts.", |scope| {
            scope.spawn((
                View,
                Node {
                    width: percent(100),
                    height: px(180),
                    flex_shrink: 0.0,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.40, 0.53, 0.56)),
            ))
            .observe(
                |_: On<Pointer<DragStart>>, state: Res<GameState>, mut ui: ResMut<PeriscopeUi>| {
                    if let Some(p) = &state.periscope {
                        ui.drag_origin = p.bearing;
                    }
                },
            )
            .observe(
                |mut drag: On<Pointer<Drag>>,
                 state: Res<GameState>,
                 ui: Res<PeriscopeUi>,
                 view: Single<&ComputedNode, With<View>>,
                 mut queue: ResMut<CommandQueue>| {
                    if !ui.open {
                        return;
                    }
                    drag.propagate(false);
                    if let Some(p) = &state.periscope {
                        let width = view.size.x * view.inverse_scale_factor;
                        let bearing = (ui.drag_origin
                            - drag.distance.x / width.max(1.0) * p.field_of_view)
                            .rem_euclid(360.0);
                        queue.push(PlayerCommand::SetPeriscopeBearing(bearing));
                    }
                },
            )
            .with_children(|view| {
                crate::periscope_effects::spawn(view);
                view.spawn((
                    ClosedViewMessage,
                    Visibility::Hidden,
                    Pickable::IGNORE,
                    ZIndex(1),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        right: px(0),
                        top: px(0),
                        bottom: px(0),
                        padding: UiRect::all(px(12)),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        row_gap: px(8),
                        ..default()
                    },
                ))
                .with_children(|message| {
                    message.spawn((
                        ClosedViewTitle,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.9, 0.65)),
                        TextLayout::justify(Justify::Center),
                        Pickable::IGNORE,
                        Node {
                            max_width: percent(100),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                    message.spawn((
                        ClosedViewDetail,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextLayout::justify(Justify::Center),
                        Pickable::IGNORE,
                        Node {
                            max_width: percent(100),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                });
                view.spawn((
                    SceneGeometry,
                    Node {
                        position_type: PositionType::Absolute,
                        top: percent(60),
                        bottom: px(0),
                        width: percent(100),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.13, 0.28, 0.32)),
                    Pickable::IGNORE,
                ));
                for index in 0..16 {
                    let mut slot = view.spawn((
                        Slot(index),
                        Button,
                        Visibility::Hidden,
                        BorderColor::all(Color::srgb(0.5, 0.95, 0.65)),
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(44),
                            height: px(44),
                            ..default()
                        },
                    ));
                    button_activation::enable(&mut slot);
                    slot.with_children(|s| {
                        s.spawn((
                            Hull(index),
                            Node {
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.025, 0.035, 0.04)),
                            Pickable::IGNORE,
                        ));
                        for funnel in 0..2 {
                            s.spawn((
                                Funnel(index, funnel),
                                Node {
                                    position_type: PositionType::Absolute,
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.025, 0.035, 0.04)),
                                Pickable::IGNORE,
                            ));
                        }
                    });
                }
                for node in [
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(50),
                        height: percent(100),
                        width: px(1),
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        top: percent(60),
                        width: percent(100),
                        height: px(1),
                        ..default()
                    },
                ] {
                    view.spawn((
                        SceneGeometry,
                        node,
                        BackgroundColor(Color::srgba(0.6, 0.9, 0.75, 0.6)),
                        Pickable::IGNORE,
                    ));
                }
                crate::periscope_graticule::spawn(view);
            });
            for actions in [
                vec![
                    (Action::Left, "< -5 deg"),
                    (Action::Raise, "MONTER / BAISSER"),
                    (Action::Right, "+5 deg >"),
                ],
                vec![
                    (Action::Create, "CREER CONTACT"),
                    (Action::NextContact, "CONTACT SUIVANT"),
                    (Action::Share, "PARTAGER"),
                ],
            ] {
                action_row(scope, &actions);
            }
            scope.spawn((
                ContactStatus,
                Text::new("Touchez une silhouette"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Node { flex_shrink: 0.0, ..default() },
            ));
            });
            root.spawn((CommandPanels, Node {
                width: percent(100), flex_direction: FlexDirection::Column,
                row_gap: px(12), column_gap: px(12), flex_shrink: 0.0,
                align_items: AlignItems::Start, ..default()
            })).with_children(|orders| {
                section(orders, StationSection::Pilot, "PILOTE // MANOEUVRER", "Consignes du sous-marin, pas celles de la cible.", |pilot| {
                    pilot.spawn((PilotStatus, Text::new("CONSIGNES LOCALES"),
                        TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::WHITE),
                        Node { flex_shrink: 0.0, ..default() }));
                    action_row(pilot, &[(Action::PilotHeading(-10.0), "CAP SOUS-MARIN -10"), (Action::PilotHeading(10.0), "CAP SOUS-MARIN +10")]);
                    action_row(pilot, &[(Action::PilotSpeed(-1.0), "VITESSE SOUS-MARIN -1 kn"), (Action::PilotSpeed(1.0), "VITESSE SOUS-MARIN +1 kn")]);
                    action_row(pilot, &[(Action::PilotDepth(-2.0), "PROFONDEUR -2 m"), (Action::PilotDepth(2.0), "PROFONDEUR +2 m")]);
                    action_row(pilot, &[(Action::OrderPilot, "TRANSMETTRE AU PILOTE")]);
                });
                section(orders, StationSection::Weapons, "ARMEMENT // PREPARER L'ATTAQUE", "Hypotheses sur la cible observee au periscope.", |weapons| {
                    weapons.spawn((EstimateStatus, Text::new("ESTIMATIONS LOCALES"),
                        TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::WHITE),
                        Node { flex_shrink: 0.0, ..default() }));
                    action_row(weapons, &[(Action::Distance(-100.0), "DISTANCE CIBLE -100 m"), (Action::Distance(100.0), "DISTANCE CIBLE +100 m")]);
                    action_row(weapons, &[(Action::TargetHeading(-5.0), "CAP CIBLE -5 deg"), (Action::TargetHeading(5.0), "CAP CIBLE +5 deg")]);
                    action_row(weapons, &[(Action::TargetSpeed(-1.0), "VITESSE CIBLE -1 kn"), (Action::TargetSpeed(1.0), "VITESSE CIBLE +1 kn")]);
                    label(weapons, "Transmettre remplace la designation visuelle et annule les anciennes autorisations de tir visuelles.");
                    action_row(weapons, &[(Action::ApplyEstimate, "TRANSMETTRE A L'ARMEMENT")]);
                    weapons.spawn((AppliedEstimateStatus, Text::new("AUCUNE DESIGNATION TRANSMISE"),
                        TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::srgb(0.95, 0.80, 0.50)),
                        Node { padding: UiRect::all(px(8)), border: UiRect::all(px(1)), flex_shrink: 0.0, ..default() },
                        BorderColor::all(Color::srgb(0.45, 0.32, 0.12)), BackgroundColor(Color::srgb(0.055, 0.04, 0.02))));
                    action_row(weapons, &[(Action::OrderVisual, "ORDONNER LA PREPARATION")]);
                    label(weapons, "Puis CARTE : choisir le tube configure et AUTORISER. Avec un Armement humain, lui demander la preparation.");
                });
            });
        });
}

fn toggle_label(open: bool) -> &'static str {
    if open {
        "REVENIR A LA CARTE"
    } else {
        "POSTE DE COMMANDE"
    }
}

fn update_toggle_label(ui: Res<PeriscopeUi>, mut text: Single<&mut Text, With<ToggleLabel>>) {
    text.0 = toggle_label(ui.open).into();
}

fn scroll_panel(
    mut wheel: MessageReader<MouseWheel>,
    ui: Res<PeriscopeUi>,
    mut panels: Query<(&ComputedNode, &RelativeCursorPosition, &mut ScrollPosition), With<Panel>>,
) {
    for (node, cursor, mut scroll) in &mut panels {
        let maximum = (node.content_size.y - node.size.y).max(0.0) * node.inverse_scale_factor;
        scroll.0.y = scroll.0.y.clamp(0.0, maximum);
        for event in wheel.read() {
            if ui.open && cursor.cursor_over() {
                let delta = match event.unit {
                    MouseScrollUnit::Line => event.y * 44.0,
                    MouseScrollUnit::Pixel => event.y,
                };
                scroll.0.y = (scroll.0.y - delta).clamp(0.0, maximum);
            }
        }
    }
}

fn buttons(
    mut events: MessageReader<ButtonActivated>,
    mut actions: Query<(&Action, &mut ButtonFeedback)>,
    slots: Query<&Slot>,
    player: Res<LocalPlayer>,
    state: Res<GameState>,
    mut ui: ResMut<PeriscopeUi>,
    mut queue: ResMut<CommandQueue>,
    time: Res<Time<Real>>,
) {
    if !state.game_started || player.role != Some(CrewRole::Captain) {
        events.clear();
        return;
    }
    let Some(p) = &state.periscope else {
        events.clear();
        return;
    };
    for event in events.read() {
        if let Ok(slot) = slots.get(event.0) {
            if ui.open {
                ui.selected_observation = p.silhouettes.get(slot.0).map(|s| s.observation_id);
            }
        }
        let Ok((action, mut feedback)) = actions.get_mut(event.0) else {
            continue;
        };
        if !ui.open && !matches!(action, Action::Toggle) {
            continue;
        }
        if action_available(*action, &ui, &state, &player) {
            feedback.flash_until = time.elapsed_secs_f64() + CLICK_FLASH_SECONDS;
        } else if !(matches!(action, Action::Raise)
            && matches!(
                p.blocked,
                Some(PeriscopeBlock::Depth | PeriscopeBlock::Speed)
            ))
        {
            continue;
        }
        match *action {
            Action::Toggle => ui.open = !ui.open,
            Action::Raise => {
                ui.raise_attempted = !p.raised;
                queue.push(PlayerCommand::SetPeriscopeRaised(!p.raised));
            }
            Action::Left | Action::Right => {
                let delta = if matches!(action, Action::Left) {
                    -5.0
                } else {
                    5.0
                };
                queue.push(PlayerCommand::SetPeriscopeBearing(
                    (p.bearing + delta).rem_euclid(360.0),
                ));
            }
            Action::Create => {
                if let Some(observation_id) = ui.selected_observation {
                    ui.pending_contact = Some(observation_id);
                    queue.push(PlayerCommand::CreateVisualContact { observation_id });
                }
            }
            Action::NextContact => {
                let next = state
                    .visual_contacts
                    .iter()
                    .position(|c| Some(c.id) == ui.selected_contact)
                    .map_or(0, |i| (i + 1) % state.visual_contacts.len().max(1));
                ui.selected_contact = state.visual_contacts.get(next).map(|c| c.id);
            }
            Action::Share => {
                if let Some(id) = ui.selected_contact {
                    queue.push(PlayerCommand::ShareVisualContact(id));
                }
            }
            Action::PilotHeading(delta) => {
                ui.pilot.heading = (ui.pilot.heading + delta).rem_euclid(360.0)
            }
            Action::PilotSpeed(delta) => ui.pilot.speed = (ui.pilot.speed + delta).clamp(0.0, 18.0),
            Action::PilotDepth(delta) => {
                ui.pilot.depth = (ui.pilot.depth + delta).clamp(0.0, 250.0)
            }
            Action::OrderPilot => queue.order_pilot(ui.pilot),
            Action::Distance(delta) => ui.distance = (ui.distance + delta).clamp(100.0, 30_000.0),
            Action::TargetHeading(delta) => {
                ui.target_heading = (ui.target_heading + delta).rem_euclid(360.0)
            }
            Action::TargetSpeed(delta) => {
                ui.target_speed = (ui.target_speed + delta).clamp(0.0, 40.0)
            }
            Action::ApplyEstimate => {
                if let Some(contact) = ui.selected_contact {
                    queue.push(PlayerCommand::DesignateVisualTarget {
                        contact,
                        distance: ui.distance,
                        heading: ui.target_heading,
                        speed: ui.target_speed,
                    });
                }
            }
            Action::OrderVisual => {
                if let Some(v) = state.weapons.as_ref().and_then(|w| w.visual_target) {
                    queue.push(PlayerCommand::OrderWeaponsBot {
                        target: shared::ShotTarget::Visual(v.revision),
                    });
                }
            }
        }
    }
}

fn action_available(
    action: Action,
    ui: &PeriscopeUi,
    state: &GameState,
    player: &LocalPlayer,
) -> bool {
    if !state.game_started || player.role != Some(CrewRole::Captain) {
        return false;
    }
    if matches!(action, Action::Toggle) {
        return state.periscope.is_some();
    }
    if !ui.open {
        return false;
    }
    let Some(scope) = &state.periscope else {
        return false;
    };
    if state
        .submarine
        .as_ref()
        .is_some_and(|s| s.common.damage.destroyed)
    {
        return false;
    }
    match action {
        Action::Toggle => true,
        Action::Raise => scope.raised || scope.blocked.is_none(),
        Action::Create => {
            scope.raised
                && scope
                    .silhouettes
                    .iter()
                    .any(|s| Some(s.observation_id) == ui.selected_observation)
        }
        Action::NextContact => !state.visual_contacts.is_empty(),
        Action::Share => state
            .visual_contacts
            .iter()
            .any(|c| Some(c.id) == ui.selected_contact && !c.shared),
        Action::OrderPilot => !state.lobby.as_ref().is_some_and(|lobby| {
            lobby.slots.iter().any(|s| {
                s.role == CrewRole::Pilot
                    && matches!(s.occupant, shared::RoleOccupant::Human { .. })
            })
        }),
        Action::PilotSpeed(delta) => (ui.pilot.speed + delta).clamp(0.0, 18.0) != ui.pilot.speed,
        Action::PilotDepth(delta) => (ui.pilot.depth + delta).clamp(0.0, 250.0) != ui.pilot.depth,
        Action::PilotHeading(_) | Action::Left | Action::Right => true,
        Action::Distance(delta) => (ui.distance + delta).clamp(100.0, 30_000.0) != ui.distance,
        Action::TargetSpeed(delta) => (ui.target_speed + delta).clamp(0.0, 40.0) != ui.target_speed,
        Action::TargetHeading(_) => true,
        Action::ApplyEstimate => state
            .visual_contacts
            .iter()
            .any(|c| Some(c.id) == ui.selected_contact),
        Action::OrderVisual => {
            state
                .weapons
                .as_ref()
                .and_then(|w| w.visual_target)
                .is_some_and(|v| {
                    Some(v.contact) == ui.selected_contact
                        && v.distance == ui.distance
                        && v.heading == ui.target_heading
                        && v.speed == ui.target_speed
                })
                && state.lobby.as_ref().is_some_and(|l| {
                    l.slots.iter().any(|s| {
                        s.role == CrewRole::Weapons && s.occupant == shared::RoleOccupant::Bot
                    })
                })
        }
    }
}

fn update_estimate_status(
    ui: Res<PeriscopeUi>,
    state: Res<GameState>,
    mut texts: Query<
        (&mut Text, Has<AppliedEstimateStatus>),
        Or<(With<EstimateStatus>, With<AppliedEstimateStatus>)>,
    >,
) {
    for (mut text, applied) in &mut texts {
        if applied {
            text.0 = state.weapons.as_ref().and_then(|w| w.visual_target.map(|v| (w, v)))
            .map(|(w, v)| format!("DESIGNATION TRANSMISE // visuel {} / r{}\nDistance {:.0} m / cap cible {:.0} deg / vitesse {:.1} kn\nRelevement {:.0} deg observe a {:.1}s / age {:.0}s",
                v.contact.0, v.revision, v.distance, v.heading, v.speed, v.bearing, v.observed_at, w.visual_target_age))
            .unwrap_or_else(|| "AUCUNE DESIGNATION TRANSMISE\nChoisir un contact puis transmettre les estimations.".into());
        } else {
            text.0 = format!("ESTIMATIONS LOCALES // {}\nDistance cible {:.0} m\nCap cible {:03.0} deg / vitesse cible {:.1} kn\nHypotheses manuelles, non mesurees. Les modifications restent locales jusqu'a transmission.",
                ui.selected_contact.map_or_else(|| "aucun contact selectionne".into(), |id| format!("visuel {}", id.0)),
                ui.distance, ui.target_heading, ui.target_speed);
        }
    }
}

fn button_appearance(
    available: bool,
    interaction: Interaction,
    flashing: bool,
) -> ButtonAppearance {
    if available && interaction == Interaction::Pressed {
        return ButtonAppearance::Pressed;
    }
    // A legitimate click can itself make the action unavailable (last increment, sharing).
    if flashing {
        ButtonAppearance::Flash
    } else if !available {
        ButtonAppearance::Disabled
    } else {
        match interaction {
            Interaction::Pressed => ButtonAppearance::Pressed,
            Interaction::Hovered => ButtonAppearance::Hovered,
            Interaction::None => ButtonAppearance::Idle,
        }
    }
}

fn update_button_feedback(
    time: Res<Time<Real>>,
    ui: Res<PeriscopeUi>,
    state: Res<GameState>,
    player: Res<LocalPlayer>,
    mut buttons: Query<(
        &Action,
        &Interaction,
        &InheritedVisibility,
        &mut ButtonFeedback,
        &mut BackgroundColor,
        &mut BorderColor,
        &mut Node,
        &Children,
    )>,
    mut labels: Query<&mut TextColor>,
) {
    let now = time.elapsed_secs_f64();
    for (
        action,
        interaction,
        visible,
        mut feedback,
        mut background,
        mut border,
        mut node,
        children,
    ) in &mut buttons
    {
        if !visible.get() {
            feedback.flash_until = 0.0;
        }
        let appearance = button_appearance(
            action_available(*action, &ui, &state, &player),
            *interaction,
            now < feedback.flash_until,
        );
        let (fill, outline) = action_section(*action).colors(&appearance);
        background.set_if_neq(BackgroundColor(fill));
        border.set_if_neq(BorderColor::all(outline));
        let padding = if matches!(action, Action::Toggle) {
            8.0
        } else {
            4.0
        };
        let pressed = matches!(appearance, ButtonAppearance::Pressed);
        let inset = if pressed { 1.0 } else { 0.0 };
        let padding = UiRect {
            top: px(padding + inset),
            bottom: px(padding - inset),
            left: px(padding),
            right: px(padding),
        };
        if node.padding != padding {
            node.padding = padding;
        }
        let color = TextColor(if appearance == ButtonAppearance::Disabled {
            Color::srgb(0.35, 0.45, 0.46)
        } else {
            Color::WHITE
        });
        for child in children.iter() {
            if let Ok(mut label) = labels.get_mut(child) {
                label.set_if_neq(color);
            }
        }
    }
}

fn update(
    player: Res<LocalPlayer>,
    state: Res<GameState>,
    mut ui: ResMut<PeriscopeUi>,
    mut visibility: Query<
        (&mut Visibility, Has<Panel>, Has<Toggle>),
        Or<(With<Panel>, With<Toggle>)>,
    >,
    mut texts: Query<(
        &mut Text,
        Has<Status>,
        Has<ContactStatus>,
        Has<PilotStatus>,
        Has<ClosedViewTitle>,
        Has<ClosedViewDetail>,
    )>,
    view: Single<&ComputedNode, With<View>>,
    mut nodes: Query<
        (
            &mut Node,
            &mut Visibility,
            Option<&Slot>,
            Option<&Hull>,
            Option<&Funnel>,
            Has<SceneGeometry>,
            Has<ClosedViewMessage>,
        ),
        (Without<Panel>, Without<Toggle>),
    >,
    mut skies: Query<&mut BackgroundColor, With<View>>,
) {
    let captain = state.game_started && player.role == Some(CrewRole::Captain);
    for (mut v, panel, toggle) in &mut visibility {
        *v = if captain && (toggle || (panel && ui.open)) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let Some(p) = &state.periscope else {
        return;
    };
    if p.raised {
        ui.raise_attempted = false;
    }
    let notice = closed_view_notice(
        p,
        state.periscope_config.unwrap_or_default(),
        state
            .submarine
            .as_ref()
            .map(|s| (s.common.depth, s.common.speed)),
        ui.raise_attempted,
    );
    if let Some(token) = ui.pending_contact {
        if let Some(silhouette) = p.silhouettes.iter().find(|s| s.observation_id == token) {
            if let Some(id) = silhouette.contact_id {
                ui.selected_contact = Some(id);
                ui.pending_contact = None;
            }
        } else {
            ui.pending_contact = None;
        }
    }
    if !state
        .visual_contacts
        .iter()
        .any(|c| Some(c.id) == ui.selected_contact)
    {
        ui.selected_contact = state.visual_contacts.last().map(|c| c.id);
    }
    if !p
        .silhouettes
        .iter()
        .any(|s| Some(s.observation_id) == ui.selected_observation)
    {
        ui.selected_observation = None;
    }
    for (mut text, status, contact, pilot, closed_title, closed_detail) in &mut texts {
        if closed_title {
            text.0 = notice.as_ref().map_or(String::new(), |n| n.0.into());
        }
        if closed_detail {
            text.0 = notice.as_ref().map_or(String::new(), |n| n.1.clone());
        }
        if status {
            text.0.clear();
            if let Some(submarine) = &state.submarine {
                text.0.push_str(&format!(
                    "PROFONDEUR {:.1} m // VITESSE {:.1} kn",
                    submarine.common.depth, submarine.common.speed
                ));
            }
            if let Some(error) = &state.last_error {
                if !matches!(error, shared::ProtocolError::PeriscopeUnavailable(_)) {
                    text.0
                        .push_str(&format!("\n{}", crate::render::error_label(error)));
                }
            }
        }
        if contact {
            text.0 = format!(
                "{}\n{}",
                if ui.selected_observation.is_some() {
                    "Silhouette selectionnee : CREER CONTACT"
                } else {
                    "Touchez une silhouette pour la selectionner"
                },
                state
                    .visual_contacts
                    .iter()
                    .find(|c| Some(c.id) == ui.selected_contact)
                    .map(|c| format!(
                        "Visuel {} / {} / {:03.0} deg\nObserve a {:.1}s / age {:.0}s / {} / {}",
                        c.id.0,
                        kind_label(c.kind),
                        c.bearing,
                        c.observed_at,
                        c.age,
                        if c.old { "ANCIEN" } else { "RECENT" },
                        if c.shared { "PARTAGE" } else { "PRIVE" }
                    ))
                    .unwrap_or_else(|| "Aucun contact visuel".into())
            );
        }
        if pilot {
            text.0 = format!("CONSIGNES LOCALES DU SOUS-MARIN\nCap {:03.0} deg / vitesse {:.0} kn\nProfondeur {:.0} m\nAjuster puis transmettre au bot Pilote.\nIndisponible si le poste est occupe par un humain.", ui.pilot.heading, ui.pilot.speed, ui.pilot.depth);
        }
    }
    for mut sky in &mut skies {
        *sky = BackgroundColor(if p.raised {
            Color::srgb(0.40, 0.53, 0.56)
        } else {
            Color::BLACK
        });
    }
    let width = view.size.x * view.inverse_scale_factor;
    for (mut node, mut v, slot, hull, funnel, geometry, closed_message) in &mut nodes {
        if closed_message {
            *v = if notice.is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if geometry {
            *v = if p.raised {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        let index = slot
            .map(|s| s.0)
            .or_else(|| hull.map(|s| s.0))
            .or_else(|| funnel.map(|s| s.0));
        let Some(index) = index else {
            continue;
        };
        let Some(s) = p
            .silhouettes
            .get(index)
            .filter(|_| p.raised && ui.open && captain)
        else {
            *v = Visibility::Hidden;
            continue;
        };
        let x = width * (0.5 + delta(s.bearing, p.bearing) / p.field_of_view);
        let w = (width * s.angular_width / p.field_of_view).max(1.0);
        let h = (width * s.angular_height / p.field_of_view).max(1.0);
        *v = Visibility::Inherited;
        if slot.is_some() {
            node.border = UiRect::all(px(if ui.selected_observation == Some(s.observation_id) {
                1.0
            } else {
                0.0
            }));
            node.left = px(x - 22.0);
            node.top = px(108.0 - 44.0);
        }
        if hull.is_some() {
            node.left = px(22.0 - w * 0.5);
            node.bottom = px(0);
            node.width = px(w);
            node.height = px(h * 0.4);
        }
        if let Some(f) = funnel {
            let count = if s.kind == SilhouetteKind::TwinFunnelCargo {
                2
            } else {
                1
            };
            if f.1 >= count {
                *v = Visibility::Hidden;
            }
            node.left = px(22.0 + (f.1 as f32 - (count - 1) as f32 * 0.5) * w * 0.3 * s.aspect);
            node.bottom = px(h * 0.35);
            node.width = px((w * 0.12).max(1.0));
            node.height = px(h * 0.65);
        }
    }
}

fn delta(a: f32, b: f32) -> f32 {
    (a - b + 180.0).rem_euclid(360.0) - 180.0
}

fn closed_view_notice(
    scope: &shared::PeriscopeMeasurements,
    config: shared::PeriscopeConfig,
    measurements: Option<(f32, f32)>,
    raise_attempted: bool,
) -> Option<(&'static str, String)> {
    if scope.raised {
        return None;
    }
    if let Some(block) = scope.blocked {
        if block == PeriscopeBlock::Destroyed {
            return Some(("SOUS-MARIN PERDU", "Observation indisponible.".into()));
        }
        let title = if scope.last_auto_lowering.is_some() && !raise_attempted {
            "PERISCOPE ABAISSE AUTOMATIQUEMENT"
        } else {
            "OUVERTURE IMPOSSIBLE"
        };
        let detail = match block {
            PeriscopeBlock::Speed => format!(
                "Vitesse excessive{}\nMaximum : {:.0} kn",
                measurements
                    .map(|(_, speed)| format!(" : {speed:.1} kn"))
                    .unwrap_or_default(),
                config.max_speed
            ),
            PeriscopeBlock::Depth => format!(
                "Profondeur hors limites{}\nProfondeur autorisee : 0 a {:.0} m",
                measurements
                    .map(|(depth, _)| format!(" : {depth:.1} m"))
                    .unwrap_or_default(),
                config.depth + config.depth_tolerance
            ),
            PeriscopeBlock::Destroyed => unreachable!(),
        };
        return Some((title, detail));
    }
    Some((
        "PERISCOPE ABAISSE",
        if scope.last_auto_lowering.is_some() || raise_attempted {
            "Conditions reunies : appuyez sur MONTER.".into()
        } else {
            String::new()
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_button_reserves_content_width_for_its_label() {
        let mut app = App::new();
        app.add_systems(Startup, spawn);
        app.update();
        let world = app.world_mut();
        let buttons: Vec<_> = world
            .query::<(&Action, &Children, &Node)>()
            .iter(world)
            .filter(|(action, _, _)| !matches!(action, Action::Toggle))
            .map(|(_, children, node)| (children.to_vec(), node.min_height))
            .collect();
        assert!(!buttons.is_empty());
        for (children, min_height) in buttons {
            assert_eq!(min_height, px(44));
            let labels: Vec<_> = children
                .into_iter()
                .filter(|child| world.get::<CommandButtonLabel>(*child).is_some())
                .collect();
            assert_eq!(labels.len(), 1);
            let label = labels[0];
            assert!(!world.get::<Text>(label).unwrap().0.is_empty());
            let node = world.get::<Node>(label).unwrap();
            assert_eq!(node.width, percent(100));
            assert_eq!(node.min_width, px(0));
            assert_eq!(node.max_width, Val::Auto);
        }
    }

    #[test]
    fn command_post_toggle_has_a_reserved_width_and_labels_the_destination() {
        let mut app = App::new();
        app.init_resource::<PeriscopeUi>()
            .add_systems(Startup, spawn)
            .add_systems(Update, update_toggle_label);
        app.update();
        let world = app.world_mut();
        let (button, node) = world
            .query_filtered::<(Entity, &Node), With<Toggle>>()
            .single(world)
            .unwrap();
        assert_eq!(node.width, px(220));
        assert_eq!(node.min_width, px(180));
        assert_eq!(node.min_height, px(44));
        let (text, parent) = world
            .query_filtered::<(&Text, &ChildOf), With<ToggleLabel>>()
            .single(world)
            .unwrap();
        assert_eq!(parent.parent(), button);
        assert_eq!(text.0, "POSTE DE COMMANDE");
        world.resource_mut::<PeriscopeUi>().open = true;
        app.update();
        let world = app.world_mut();
        assert_eq!(
            world
                .query_filtered::<&Text, With<ToggleLabel>>()
                .single(world)
                .unwrap()
                .0,
            "REVENIR A LA CARTE"
        );
    }

    #[test]
    fn commands_belong_to_their_own_named_section() {
        let mut app = App::new();
        app.add_systems(Startup, spawn);
        app.update();
        let world = app.world_mut();
        assert_eq!(world.query::<&StationSection>().iter(world).count(), 3);
        let actions: Vec<_> = world
            .query::<(Entity, &Action)>()
            .iter(world)
            .map(|(e, a)| (e, *a))
            .collect();
        for (mut entity, action) in actions {
            if matches!(action, Action::Toggle) {
                continue;
            }
            loop {
                if let Some(section) = world.get::<StationSection>(entity) {
                    assert_eq!(*section, action_section(action));
                    break;
                }
                entity = world
                    .get::<ChildOf>(entity)
                    .expect("command without section")
                    .parent();
            }
        }
        assert_eq!(
            world
                .query_filtered::<Entity, With<AppliedEstimateStatus>>()
                .iter(world)
                .count(),
            1
        );
        assert_ne!(
            StationSection::Pilot.colors(&ButtonAppearance::Idle),
            StationSection::Weapons.colors(&ButtonAppearance::Idle)
        );
        assert_ne!(
            StationSection::Periscope.colors(&ButtonAppearance::Idle),
            StationSection::Weapons.colors(&ButtonAppearance::Idle)
        );
    }

    #[test]
    fn command_cards_stack_at_360px_and_split_on_large_screens() {
        let mut app = App::new();
        let window = app
            .world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow))
            .id();
        app.add_systems(Startup, spawn)
            .add_systems(Update, update_command_layout);
        for (width, expected) in [
            (360.0, FlexDirection::Column),
            (640.0, FlexDirection::Column),
            (960.0, FlexDirection::Row),
            (1920.0, FlexDirection::Row),
            (360.0, FlexDirection::Column),
        ] {
            app.world_mut()
                .get_mut::<Window>(window)
                .unwrap()
                .resolution
                .set(width, 640.0);
            app.update();
            let world = app.world_mut();
            let mut panels = world.query_filtered::<&Node, With<CommandPanels>>();
            assert_eq!(panels.single(world).unwrap().flex_direction, expected);
            for (node, section) in world.query::<(&Node, &StationSection)>().iter(world) {
                if *section != StationSection::Periscope {
                    assert_eq!(
                        node.width,
                        if expected == FlexDirection::Row {
                            Val::Auto
                        } else {
                            percent(100)
                        }
                    );
                }
            }
        }
    }

    #[test]
    fn button_feedback_distinguishes_hover_hold_flash_and_disabled() {
        assert_eq!(
            button_appearance(true, Interaction::None, false),
            ButtonAppearance::Idle
        );
        assert_eq!(
            button_appearance(true, Interaction::Hovered, false),
            ButtonAppearance::Hovered
        );
        assert_eq!(
            button_appearance(true, Interaction::Pressed, false),
            ButtonAppearance::Pressed
        );
        assert_eq!(
            button_appearance(true, Interaction::Pressed, true),
            ButtonAppearance::Pressed
        );
        assert_eq!(
            button_appearance(false, Interaction::Pressed, false),
            ButtonAppearance::Disabled
        );
        // Valid clicks stay visible even if their action becomes unavailable immediately.
        assert_eq!(
            button_appearance(false, Interaction::None, true),
            ButtonAppearance::Flash
        );
        let clicked_at = 10.0;
        let flash_until = clicked_at + CLICK_FLASH_SECONDS;
        assert_eq!(
            button_appearance(true, Interaction::None, clicked_at + 0.10 < flash_until),
            ButtonAppearance::Flash
        );
        assert_eq!(
            button_appearance(true, Interaction::None, clicked_at + 0.20 < flash_until),
            ButtonAppearance::Idle
        );
    }

    #[test]
    fn unavailable_actions_have_no_misleading_click_feedback() {
        let mut state = GameState {
            game_started: true,
            periscope: Some(scope()),
            ..default()
        };
        let mut ui = PeriscopeUi {
            open: true,
            ..default()
        };
        let mut player = LocalPlayer::default();
        player.role = Some(CrewRole::Captain);
        assert!(action_available(Action::Raise, &ui, &state, &player));
        assert!(!action_available(Action::Create, &ui, &state, &player));
        assert!(!action_available(Action::NextContact, &ui, &state, &player));
        assert!(!action_available(Action::Share, &ui, &state, &player));
        state.periscope.as_mut().unwrap().blocked = Some(PeriscopeBlock::Speed);
        assert!(!action_available(Action::Raise, &ui, &state, &player));
        state.periscope.as_mut().unwrap().raised = true;
        assert!(action_available(Action::Raise, &ui, &state, &player));
        ui.pilot.speed = 0.0;
        assert!(!action_available(
            Action::PilotSpeed(-1.0),
            &ui,
            &state,
            &player
        ));
        assert!(action_available(
            Action::PilotSpeed(1.0),
            &ui,
            &state,
            &player
        ));
        ui.pilot.depth = 250.0;
        assert!(!action_available(
            Action::PilotDepth(2.0),
            &ui,
            &state,
            &player
        ));
        assert!(action_available(
            Action::PilotDepth(-2.0),
            &ui,
            &state,
            &player
        ));
        state.lobby = Some(shared::LobbySnapshot {
            room_id: shared::RoomId("000001".into()),
            phase: shared::LobbyPhase::Running,
            slots: vec![shared::RoleSlot {
                role: CrewRole::Pilot,
                occupant: shared::RoleOccupant::Human {
                    player_id: shared::PlayerId(2),
                    ready: true,
                },
            }],
        });
        assert!(!action_available(Action::OrderPilot, &ui, &state, &player));
    }

    fn scope() -> shared::PeriscopeMeasurements {
        shared::PeriscopeMeasurements {
            raised: false,
            bearing: 0.0,
            field_of_view: 40.0,
            blocked: None,
            last_auto_lowering: None,
            silhouettes: vec![],
        }
    }

    #[test]
    fn closed_view_notice_distinguishes_manual_lowering_automatic_lowering_and_refusal() {
        let mut p = scope();
        let c = shared::PeriscopeConfig::default();
        assert_eq!(
            closed_view_notice(&p, c, None, false),
            Some(("PERISCOPE ABAISSE", String::new()))
        );
        p.blocked = Some(PeriscopeBlock::Speed);
        let refusal = closed_view_notice(&p, c, Some((0.0, 7.0)), true).unwrap();
        assert_eq!(refusal.0, "OUVERTURE IMPOSSIBLE");
        assert!(refusal.1.contains("7.0 kn") && refusal.1.contains("6 kn"));
        p.blocked = None;
        assert!(closed_view_notice(&p, c, None, true)
            .unwrap()
            .1
            .contains("Conditions reunies"));
        p.blocked = Some(PeriscopeBlock::Speed);
        p.last_auto_lowering = p.blocked;
        assert_eq!(
            closed_view_notice(&p, c, None, false).unwrap().0,
            "PERISCOPE ABAISSE AUTOMATIQUEMENT"
        );
        assert_eq!(
            closed_view_notice(&p, c, None, true).unwrap().0,
            "OUVERTURE IMPOSSIBLE"
        );
        p.blocked = None;
        let ready = closed_view_notice(&p, c, None, false).unwrap();
        assert_eq!(ready.0, "PERISCOPE ABAISSE");
        assert!(ready.1.contains("Conditions reunies"));
        assert!(!ready.1.contains("Vitesse excessive"));
        p.raised = true;
        assert!(closed_view_notice(&p, c, None, false).is_none());
    }

    #[test]
    fn closed_view_notice_uses_mission_limits_and_prioritizes_destruction() {
        let mut p = scope();
        let c = shared::PeriscopeConfig {
            depth: 15.0,
            depth_tolerance: 1.0,
            max_speed: 4.0,
            ..default()
        };
        p.blocked = Some(PeriscopeBlock::Depth);
        assert!(closed_view_notice(&p, c, Some((18.0, 0.0)), true)
            .unwrap()
            .1
            .contains("0 a 16 m"));
        p.blocked = Some(PeriscopeBlock::Speed);
        assert!(closed_view_notice(&p, c, None, true)
            .unwrap()
            .1
            .contains("4 kn"));
        p.blocked = Some(PeriscopeBlock::Destroyed);
        assert_eq!(
            closed_view_notice(&p, c, None, true).unwrap().0,
            "SOUS-MARIN PERDU"
        );
    }
}
pub(crate) fn block_label(block: PeriscopeBlock) -> &'static str {
    match block {
        PeriscopeBlock::Depth => "PROFONDEUR HORS LIMITES",
        PeriscopeBlock::Speed => "VITESSE EXCESSIVE",
        PeriscopeBlock::Destroyed => "SOUS-MARIN PERDU",
    }
}
pub(crate) fn kind_label(kind: SilhouetteKind) -> &'static str {
    match kind {
        SilhouetteKind::SingleFunnelCargo => "une cheminee",
        SilhouetteKind::TwinFunnelCargo => "deux cheminees",
        SilhouetteKind::Escort => "silhouette basse",
    }
}
pub(crate) fn shared_contacts_text(state: &GameState) -> String {
    state
        .visual_contacts
        .iter()
        .filter(|c| c.shared)
        .map(|c| {
            format!(
                "\nVISUEL {} / {} / {:03.0} deg / t{:.0}s / age {:.0}s{}",
                c.id.0,
                kind_label(c.kind),
                c.bearing,
                c.observed_at,
                c.age,
                if c.old { " ANCIEN" } else { "" }
            )
        })
        .collect()
}

pub(crate) fn spawn_shared_contacts(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        SharedContacts,
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::srgb(0.7, 0.95, 0.8)),
        Node {
            flex_shrink: 0.0,
            ..default()
        },
    ));
}

fn update_shared_contacts(
    state: Res<GameState>,
    mut lists: Query<&mut Text, With<SharedContacts>>,
) {
    for mut list in &mut lists {
        list.0 = shared_contacts_text(&state);
    }
}
