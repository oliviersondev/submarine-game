use bevy::prelude::*;
use shared::{CrewRole, DamageControlAction, EngineeringMeasurements, PlayerCommand, SystemId};

use crate::button_activation::{self, ButtonActivated};
use crate::network::{CommandQueue, GameState, LocalPlayer, NetworkReceiveSet};

#[derive(Resource, Default)]
struct Selection(u8);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Action {
    Select(u8),
    Pump,
    Isolate,
    Seal,
    Extinguish,
    Repair(u8),
    Cancel,
}

pub struct DamageControlUiPlugin;
impl Plugin for DamageControlUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Selection>()
            .add_systems(Update, (buttons, labels).chain().after(NetworkReceiveSet));
    }
}

pub(crate) fn spawn(parent: &mut ChildSpawnerCommands) {
    for row in [
        vec![Action::Select(0), Action::Select(1), Action::Select(2)],
        vec![Action::Pump, Action::Isolate],
        vec![Action::Seal, Action::Extinguish],
        vec![Action::Repair(0), Action::Repair(1)],
        vec![Action::Cancel],
    ] {
        parent
            .spawn(Node {
                width: percent(100),
                column_gap: px(4),
                flex_shrink: 0.0,
                ..default()
            })
            .with_children(|line| {
                for action in row {
                    let mut button = line.spawn((
                        Button,
                        action,
                        Text::new("---"),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Node {
                            min_height: px(44),
                            flex_grow: 1.0,
                            flex_basis: px(0),
                            padding: UiRect::all(px(6)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.12, 0.16, 0.12)),
                        BackgroundGradient::default(),
                    ));
                    button_activation::enable(&mut button);
                }
            });
    }
}

fn systems(compartment: u8) -> Vec<SystemId> {
    SystemId::ALL
        .into_iter()
        .filter(|id| id.compartment() == compartment)
        .collect()
}

fn command(
    action: Action,
    selected: u8,
    engineering: &EngineeringMeasurements,
) -> Option<PlayerCommand> {
    let c = engineering.compartments.get(selected as usize)?;
    Some(match action {
        Action::Select(_) => return None,
        Action::Pump => PlayerCommand::SetCompartmentPump {
            compartment: selected,
            enabled: !c.pumping,
        },
        Action::Isolate => PlayerCommand::SetCompartmentIsolated {
            compartment: selected,
            isolated: !c.isolated,
        },
        Action::Seal => PlayerCommand::StartDamageControl {
            compartment: selected,
            action: DamageControlAction::SealBreach,
        },
        Action::Extinguish => PlayerCommand::StartDamageControl {
            compartment: selected,
            action: DamageControlAction::Extinguish,
        },
        Action::Repair(index) => {
            PlayerCommand::RepairSystem(*systems(selected).get(index as usize)?)
        }
        Action::Cancel => PlayerCommand::CancelDamageControl,
    })
}

fn buttons(
    mut activated: MessageReader<ButtonActivated>,
    query: Query<&Action>,
    player: Res<LocalPlayer>,
    state: Res<GameState>,
    mut selection: ResMut<Selection>,
    mut queue: ResMut<CommandQueue>,
) {
    if !state.game_started || player.role != Some(CrewRole::Engineer) {
        activated.clear();
        return;
    }
    let Some(engineering) = state
        .submarine
        .as_ref()
        .and_then(|s| s.engineering.as_ref())
    else {
        activated.clear();
        return;
    };
    for event in activated.read() {
        let Ok(action) = query.get(event.0) else {
            continue;
        };
        if let Action::Select(index) = action {
            selection.0 = *index;
        } else if let Some(command) = command(*action, selection.0, engineering) {
            queue.push(command);
        }
    }
}

fn labels(
    state: Res<GameState>,
    selection: Res<Selection>,
    mut query: Query<(
        &Action,
        &mut Text,
        &mut BackgroundColor,
        &mut Node,
        &mut BackgroundGradient,
    )>,
) {
    if !state.is_changed() && !selection.is_changed() {
        return;
    }
    let Some(engineering) = state
        .submarine
        .as_ref()
        .and_then(|s| s.engineering.as_ref())
    else {
        return;
    };
    let c = engineering.compartments[selection.0 as usize];
    let lost = state
        .submarine
        .as_ref()
        .is_some_and(|s| s.common.damage.destroyed);
    for (action, mut text, mut color, mut node, mut gradient) in &mut query {
        let display = if matches!(action, Action::Repair(i) if systems(selection.0).get(*i as usize).is_none())
        {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
        let active = match action {
            Action::Select(i) => selection.0 == *i,
            Action::Pump => c.pumping,
            Action::Isolate => c.isolated,
            _ => false,
        };
        let intervention = if lost {
            None
        } else {
            matching_intervention(*action, selection.0, engineering)
        };
        let mut label = match action {
            Action::Select(i) => format!(
                "{}{}",
                if active { "> " } else { "" },
                ["AVANT", "CENTRAL", "MACHINES"][*i as usize]
            ),
            Action::Pump => format!("POMPE {}", if c.pumping { "ARRETER" } else { "DEMARRER" }),
            Action::Isolate => {
                if c.isolated {
                    "ROUVRIR".into()
                } else {
                    "ISOLER".into()
                }
            }
            Action::Seal => "COLMATER".into(),
            Action::Extinguish => "ETEINDRE".into(),
            Action::Repair(i) => systems(selection.0)
                .get(*i as usize)
                .map(|id| format!("REPARER {}", system_label(*id)))
                .unwrap_or("---".into()),
            Action::Cancel => "ANNULER INTERVENTION".into(),
        };
        let mut background = if active || intervention.is_some() {
            Color::srgb(0.25, 0.32, 0.12)
        } else {
            Color::srgb(0.08, 0.15, 0.18)
        };
        let progress_gradient = if let Some(task) = intervention {
            let progress = intervention_percent(task);
            let fill = if task.paused {
                background = Color::srgb(0.28, 0.15, 0.04);
                Color::srgb(0.62, 0.33, 0.07)
            } else {
                Color::srgb(0.38, 0.49, 0.17)
            };
            label.push_str(&format!(
                " - {progress:.0}%{}",
                if task.paused { " / SUSPENDU" } else { "" }
            ));
            progress_background(progress, fill, background)
        } else {
            BackgroundGradient::default()
        };
        text.set_if_neq(Text::new(label));
        color.set_if_neq(BackgroundColor(background));
        gradient.set_if_neq(progress_gradient);
    }
}

fn matching_intervention(
    action: Action,
    selected: u8,
    engineering: &EngineeringMeasurements,
) -> Option<shared::Intervention> {
    let task = engineering.damage_control.intervention?;
    if task.compartment != selected {
        return None;
    }
    let target = match action {
        Action::Seal => DamageControlAction::SealBreach,
        Action::Extinguish => DamageControlAction::Extinguish,
        Action::Repair(index) => {
            DamageControlAction::RepairSystem(*systems(selected).get(index as usize)?)
        }
        _ => return None,
    };
    (task.action == target).then_some(task)
}

fn intervention_percent(task: shared::Intervention) -> f32 {
    if !task.duration.is_finite() || task.duration <= 0.0 || !task.remaining.is_finite() {
        return 0.0;
    }
    ((1.0 - task.remaining / task.duration) * 100.0).clamp(0.0, 100.0)
}

fn progress_background(progress: f32, fill: Color, base: Color) -> BackgroundGradient {
    // Coincident stops give a solid fill, not a fading gradient. Keeping it on
    // the button itself leaves text and pointer/drag handling unchanged.
    BackgroundGradient::from(LinearGradient::to_right(vec![
        ColorStop::new(fill, percent(0)),
        ColorStop::new(fill, percent(progress)),
        ColorStop::new(base, percent(progress)),
        ColorStop::new(base, percent(100)),
    ]))
}

pub(crate) fn system_label(id: SystemId) -> &'static str {
    match id {
        SystemId::Engine => "MOTEURS",
        SystemId::Torpedo => "TUBES",
        SystemId::Sonar => "SONAR",
        SystemId::Life => "VENTILATION",
        SystemId::Navigation => "GOUVERNES",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engineering() -> EngineeringMeasurements {
        EngineeringMeasurements {
            propulsion: shared::PropulsionState::default(),
            battery: 100.0,
            oxygen: 100.0,
            electrical_load: 0.0,
            air_intake_available: true,
            compartments: [shared::CompartmentState::default(); 3],
            damage_history: vec![],
            damage_control: shared::DamageControlStatus::default(),
        }
    }

    fn task(action: DamageControlAction) -> shared::Intervention {
        shared::Intervention {
            compartment: 2,
            action,
            remaining: 13.5,
            duration: 18.0,
            paused: false,
        }
    }

    #[test]
    fn percentage_uses_completed_authoritative_time_and_stays_bounded() {
        let mut task = task(DamageControlAction::Extinguish);
        assert_eq!(intervention_percent(task), 25.0);
        task.remaining = task.duration;
        assert_eq!(intervention_percent(task), 0.0);
        task.remaining = -1.0;
        assert_eq!(intervention_percent(task), 100.0);
        task.remaining = 40.0;
        assert_eq!(intervention_percent(task), 0.0);
        task.duration = 0.0;
        assert_eq!(intervention_percent(task), 0.0);
        task.duration = f32::NAN;
        assert_eq!(intervention_percent(task), 0.0);
        task.duration = 18.0;
        task.remaining = f32::INFINITY;
        assert_eq!(intervention_percent(task), 0.0);
    }

    #[test]
    fn only_the_matching_action_compartment_and_system_are_active() {
        let mut engineering = engineering();
        engineering.damage_control.intervention = Some(task(DamageControlAction::Extinguish));
        assert!(matching_intervention(Action::Extinguish, 2, &engineering).is_some());
        assert!(matching_intervention(Action::Extinguish, 1, &engineering).is_none());
        assert!(matching_intervention(Action::Seal, 2, &engineering).is_none());
        assert!(matching_intervention(Action::Cancel, 2, &engineering).is_none());
        engineering.damage_control.intervention =
            Some(task(DamageControlAction::RepairSystem(SystemId::Life)));
        assert!(matching_intervention(Action::Repair(1), 2, &engineering).is_some());
        assert!(matching_intervention(Action::Repair(0), 2, &engineering).is_none());
        assert!(matching_intervention(Action::Repair(1), 0, &engineering).is_none());
        engineering.damage_control.intervention = Some(task(DamageControlAction::SealBreach));
        assert!(matching_intervention(Action::Seal, 2, &engineering).is_some());
    }

    #[test]
    fn buttons_display_progress_pause_and_clear_it_after_cancellation_or_loss() {
        let mut engineering = engineering();
        engineering.damage_control.intervention = Some(task(DamageControlAction::Extinguish));
        let mut app = App::new();
        app.insert_resource(Selection(2))
            .insert_resource(GameState {
                submarine: Some(shared::SubmarineSnapshot {
                    common: shared::CommonMeasurements {
                        x: 0.0,
                        y: 0.0,
                        heading: 0.0,
                        speed: 0.0,
                        depth: 100.0,
                        dive_state: shared::DiveState::Submerged,
                        acoustic_level: shared::AcousticLevel::Silent,
                        alerts: vec![],
                        damage: shared::DamageSummary::default(),
                    },
                    pilot: None,
                    engineering: Some(engineering),
                }),
                ..default()
            })
            .add_systems(Startup, |mut commands: Commands| {
                commands.spawn(Node::default()).with_children(spawn);
            })
            .add_systems(Update, labels);
        app.update();
        for paused in [false, true] {
            app.world_mut()
                .resource_mut::<GameState>()
                .submarine
                .as_mut()
                .unwrap()
                .engineering
                .as_mut()
                .unwrap()
                .damage_control
                .intervention
                .as_mut()
                .unwrap()
                .paused = paused;
            app.update();
            let world = app.world_mut();
            let mut buttons =
                world.query::<(&Action, &Text, &BackgroundColor, &BackgroundGradient)>();
            for (action, label, background, gradient) in buttons.iter(world) {
                if *action == Action::Extinguish {
                    assert!(label.0.contains("25%"));
                    assert_eq!(label.0.contains("SUSPENDU"), paused);
                    assert_eq!(gradient.0.len(), 1);
                    assert_eq!(
                        background.0,
                        if paused {
                            Color::srgb(0.28, 0.15, 0.04)
                        } else {
                            Color::srgb(0.25, 0.32, 0.12)
                        }
                    );
                } else {
                    assert!(gradient.0.is_empty());
                }
            }
        }
        for lost in [false, true] {
            let mut state = app.world_mut().resource_mut::<GameState>();
            let submarine = state.submarine.as_mut().unwrap();
            submarine.common.damage.destroyed = lost;
            submarine
                .engineering
                .as_mut()
                .unwrap()
                .damage_control
                .intervention = if lost {
                Some(task(DamageControlAction::Extinguish))
            } else {
                None
            };
            app.update();
            let world = app.world_mut();
            let mut buttons = world.query::<(&Action, &Text, &BackgroundGradient)>();
            for (action, label, gradient) in buttons.iter(world) {
                assert!(gradient.0.is_empty());
                if *action == Action::Extinguish {
                    assert_eq!(label.0, "ETEINDRE");
                }
            }
        }
    }
    #[test]
    fn compartment_clicks_are_processed_in_event_order_including_a_same_frame_burst() {
        let mut player = LocalPlayer::default();
        player.role = Some(CrewRole::Engineer);
        let mut app = App::new();
        app.add_plugins(button_activation::ButtonActivationPlugin)
            .init_resource::<Selection>()
            .init_resource::<CommandQueue>()
            .insert_resource(player)
            .insert_resource(GameState {
                game_started: true,
                submarine: Some(shared::SubmarineSnapshot {
                    common: shared::CommonMeasurements {
                        x: 0.0,
                        y: 0.0,
                        heading: 0.0,
                        speed: 0.0,
                        depth: 0.0,
                        dive_state: shared::DiveState::Surface,
                        acoustic_level: shared::AcousticLevel::Silent,
                        alerts: vec![],
                        damage: shared::DamageSummary::default(),
                    },
                    pilot: None,
                    engineering: Some(engineering()),
                }),
                ..default()
            })
            .add_systems(Update, buttons);
        // Spawn in the opposite order to the actual click order.
        let machines = app.world_mut().spawn(Action::Select(2)).id();
        let central = app.world_mut().spawn(Action::Select(1)).id();
        for entity in [central, machines] {
            app.world_mut()
                .resource_mut::<Messages<ButtonActivated>>()
                .write(ButtonActivated(entity));
        }
        app.update();
        assert_eq!(app.world().resource::<Selection>().0, 2);
        app.world_mut()
            .resource_mut::<Messages<ButtonActivated>>()
            .write(ButtonActivated(central));
        app.update();
        assert_eq!(app.world().resource::<Selection>().0, 1);
    }

    #[test]
    fn compartment_selection_targets_the_right_systems() {
        assert_eq!(systems(0), vec![SystemId::Torpedo]);
        assert_eq!(systems(1), vec![SystemId::Sonar, SystemId::Navigation]);
        assert_eq!(systems(2), vec![SystemId::Engine, SystemId::Life]);
    }

    #[test]
    fn controls_use_authoritative_compartment_state_and_never_select_a_wrong_system() {
        let mut engineering = EngineeringMeasurements {
            propulsion: shared::PropulsionState::default(),
            battery: 100.0,
            oxygen: 100.0,
            electrical_load: 0.0,
            air_intake_available: true,
            compartments: [shared::CompartmentState::default(); 3],
            damage_history: vec![],
            damage_control: shared::DamageControlStatus::default(),
        };
        assert_eq!(
            command(Action::Pump, 2, &engineering),
            Some(PlayerCommand::SetCompartmentPump {
                compartment: 2,
                enabled: true
            })
        );
        engineering.compartments[2].pumping = true;
        assert_eq!(
            command(Action::Pump, 2, &engineering),
            Some(PlayerCommand::SetCompartmentPump {
                compartment: 2,
                enabled: false
            })
        );
        assert_eq!(command(Action::Repair(1), 0, &engineering), None);
        assert_eq!(
            command(Action::Repair(1), 1, &engineering),
            Some(PlayerCommand::RepairSystem(SystemId::Navigation))
        );
        assert_eq!(
            command(Action::Seal, 1, &engineering),
            Some(PlayerCommand::StartDamageControl {
                compartment: 1,
                action: DamageControlAction::SealBreach
            })
        );
    }
}
