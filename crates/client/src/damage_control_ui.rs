use bevy::prelude::*;
use shared::{CrewRole, DamageControlAction, EngineeringMeasurements, PlayerCommand, SystemId};

use crate::network::{CommandQueue, GameState, LocalPlayer, NetworkReceiveSet};

#[derive(Resource, Default)]
struct Selection(u8);

#[derive(Component, Clone, Copy)]
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
                    line.spawn((
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
                    ));
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
    query: Query<(&Interaction, &Action), Changed<Interaction>>,
    player: Res<LocalPlayer>,
    state: Res<GameState>,
    mut selection: ResMut<Selection>,
    mut queue: ResMut<CommandQueue>,
) {
    if !state.game_started || player.role != Some(CrewRole::Engineer) {
        return;
    }
    let Some(engineering) = state
        .submarine
        .as_ref()
        .and_then(|s| s.engineering.as_ref())
    else {
        return;
    };
    for (interaction, action) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
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
    mut query: Query<(&Action, &mut Text, &mut BackgroundColor, &mut Node)>,
) {
    let Some(engineering) = state
        .submarine
        .as_ref()
        .and_then(|s| s.engineering.as_ref())
    else {
        return;
    };
    let c = engineering.compartments[selection.0 as usize];
    for (action, mut text, mut color, mut node) in &mut query {
        node.display = if matches!(action, Action::Repair(i) if systems(selection.0).get(*i as usize).is_none())
        {
            Display::None
        } else {
            Display::Flex
        };
        let active = match action {
            Action::Select(i) => selection.0 == *i,
            Action::Pump => c.pumping,
            Action::Isolate => c.isolated,
            _ => false,
        };
        text.0 = match action {
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
        color.0 = if active {
            Color::srgb(0.25, 0.32, 0.12)
        } else {
            Color::srgb(0.08, 0.15, 0.18)
        };
    }
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
