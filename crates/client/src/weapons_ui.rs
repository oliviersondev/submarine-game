use crate::network::{CommandQueue, GameState, LocalPlayer};
use bevy::prelude::*;
use shared::{CrewRole, PlayerCommand, ShotSettings, TrackId, TubePhase};

#[derive(Resource)]
struct WeaponUi {
    tube: usize,
    target: Option<TrackId>,
    depth: f32,
    heading: f32,
    speed: f32,
    field: u8,
    confirmation: Option<(u8, ShotSettings)>,
    notice: String,
}

impl Default for WeaponUi {
    fn default() -> Self {
        Self {
            tube: 0,
            target: None,
            depth: 4.0,
            heading: 0.0,
            speed: 0.0,
            field: 0,
            confirmation: None,
            notice: String::new(),
        }
    }
}

#[derive(Component, Clone, Copy)]
enum Action {
    Target,
    Tube,
    Field,
    Minus,
    Plus,
    Configure,
    Prepare,
    Fire,
    Cancel,
}
#[derive(Component)]
struct WeaponStatus;
#[derive(Component)]
struct ActionLabel(Action);
#[derive(Component)]
struct WeaponScroll;
#[derive(Component)]
struct ScrollStart(Vec2);

pub struct WeaponsUiPlugin;
impl Plugin for WeaponsUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponUi>().add_systems(
            Update,
            (buttons, update)
                .chain()
                .after(crate::network::NetworkReceiveSet),
        );
    }
}

pub fn spawn(parent: &mut ChildSpawnerCommands) {
    parent.spawn((WeaponScroll, ScrollStart(Vec2::ZERO), ScrollPosition(Vec2::ZERO),
        Node { flex_direction: FlexDirection::Column, row_gap: px(4), min_height: px(0),
        overflow: Overflow::scroll_y(), min_width: px(174), flex_grow: 1.0, flex_basis: px(0), ..default() }))
        .observe(|drag: On<Pointer<Drag>>, scale: Res<UiScale>, mut panel: Single<(&ComputedNode, &mut ScrollPosition, &ScrollStart), With<WeaponScroll>>| {
            let range = (panel.0.content_size.y - panel.0.size.y).max(0.0) * panel.0.inverse_scale_factor;
            panel.1.0.y = (panel.2.0.y - drag.distance.y / scale.0).clamp(0.0, range);
        })
        .observe(|_: On<Pointer<DragStart>>, mut panel: Single<(&ScrollPosition, &mut ScrollStart), With<WeaponScroll>>| { panel.1.0 = panel.0.0; })
        .with_children(|panel| {
        panel.spawn((WeaponStatus, Text::new("ARMEMENT"), TextFont { font_size: FontSize::Px(12.0), ..default() },
            TextColor(Color::srgb(0.8, 0.95, 1.0))));
        for row in [vec![Action::Target, Action::Tube], vec![Action::Field, Action::Minus, Action::Plus],
            vec![Action::Configure, Action::Prepare], vec![Action::Fire, Action::Cancel]] {
            panel.spawn(Node { column_gap: px(4), width: percent(100), flex_shrink: 0.0, ..default() }).with_children(|line| {
                for action in row {
                    line.spawn((Button, action, Node { min_height: px(44), flex_grow: 1.0, flex_basis: px(0),
                        justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                        padding: UiRect::all(px(4)), ..default() }, BackgroundColor(Color::srgb(0.04, 0.18, 0.23))))
                    .with_children(|button| { button.spawn((ActionLabel(action), Text::new("..."),
                        TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::WHITE))); });
                }
            });
        }
    });
}

fn buttons(
    query: Query<(&Interaction, &Action), Changed<Interaction>>,
    player: Res<LocalPlayer>,
    state: Res<GameState>,
    mut ui: ResMut<WeaponUi>,
    mut commands: ResMut<CommandQueue>,
) {
    if !state.game_started {
        return;
    }
    let Some(role @ (CrewRole::Captain | CrewRole::Weapons)) = player.role else {
        return;
    };
    let Some(weapons) = &state.weapons else {
        return;
    };
    let Some(tactical) = &state.tactical else {
        return;
    };
    for (interaction, action) in &query {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if !matches!(action, Action::Fire) {
            ui.confirmation = None;
        }
        let tube = &weapons.tubes[ui.tube];
        match action {
            Action::Tube => {
                ui.tube = (ui.tube + 1) % weapons.tubes.len();
                if let Some(settings) = weapons.tubes[ui.tube].settings {
                    ui.target = Some(settings.target);
                    ui.depth = settings.depth;
                    ui.heading = settings.heading_correction;
                    ui.speed = settings.speed_correction;
                }
            }
            Action::Target => {
                let index = tactical
                    .shared_tracks
                    .iter()
                    .position(|t| Some(t.id) == ui.target)
                    .map_or(0, |i| (i + 1) % tactical.shared_tracks.len());
                ui.target = tactical.shared_tracks.get(index).map(|t| t.id);
            }
            Action::Field => ui.field = (ui.field + 1) % 3,
            Action::Minus | Action::Plus if role == CrewRole::Weapons => {
                let delta = if matches!(action, Action::Minus) {
                    -1.0
                } else {
                    1.0
                };
                match ui.field {
                    0 => ui.depth = (ui.depth + delta).clamp(1.0, 15.0),
                    1 => ui.heading = (ui.heading + delta * 5.0).clamp(-45.0, 45.0),
                    _ => ui.speed = (ui.speed + delta).clamp(-10.0, 10.0),
                }
            }
            Action::Configure => {
                let Some(target) = ui.target else {
                    ui.notice = "Choisir une piste partagee".into();
                    continue;
                };
                if role == CrewRole::Captain {
                    commands.push(PlayerCommand::OrderWeaponsBot { target });
                } else {
                    commands.push(PlayerCommand::ConfigureShot {
                        tube: ui.tube as u8,
                        settings: ShotSettings {
                            target,
                            depth: ui.depth,
                            heading_correction: ui.heading,
                            speed_correction: ui.speed,
                        },
                    });
                }
                ui.notice = "Commande envoyee".into();
            }
            Action::Prepare if role == CrewRole::Captain => {
                let Some(settings) = tube.settings else {
                    ui.notice = "Tube sans configuration".into();
                    continue;
                };
                commands.push(PlayerCommand::AuthorizeShot {
                    tube: ui.tube as u8,
                    settings,
                });
                ui.notice = "Autorisation envoyee".into();
            }
            Action::Prepare => {
                let command = match tube.phase {
                    TubePhase::Empty => PlayerCommand::LoadTube(ui.tube as u8),
                    TubePhase::Loaded => PlayerCommand::FloodTube(ui.tube as u8),
                    TubePhase::Flooded if tube.settings.is_some() => {
                        PlayerCommand::OpenTube(ui.tube as u8)
                    }
                    TubePhase::Flooded => PlayerCommand::PurgeTube(ui.tube as u8),
                    _ => {
                        ui.notice = "Operation en cours ou tube deja ouvert".into();
                        continue;
                    }
                };
                commands.push(command);
                ui.notice = "Preparation envoyee".into();
            }
            Action::Cancel if role == CrewRole::Weapons => {
                let command = match tube.phase {
                    TubePhase::Ready => PlayerCommand::CloseTube(ui.tube as u8),
                    TubePhase::Flooded => PlayerCommand::PurgeTube(ui.tube as u8),
                    _ => {
                        ui.notice = "Fermer avant de purger".into();
                        continue;
                    }
                };
                commands.push(command);
            }
            Action::Fire if role == CrewRole::Weapons => {
                let Some(settings) = tube.settings else {
                    continue;
                };
                if tube.phase != TubePhase::Ready
                    || !tube.authorized
                    || !weapons.solutions[ui.tube]
                        .as_ref()
                        .is_some_and(|s| s.launch_allowed)
                {
                    ui.notice = "Tir bloque : preparation / autorisation / solution".into();
                    continue;
                }
                let shot = (ui.tube as u8, settings);
                if ui.confirmation == Some(shot) {
                    commands.push(PlayerCommand::FireTorpedo {
                        tube: ui.tube as u8,
                    });
                    ui.confirmation = None;
                    ui.notice = "Tir envoye".into();
                } else {
                    ui.confirmation = Some(shot);
                    ui.notice = "Appuyer CONFIRMER TIR pour lancer".into();
                }
            }
            _ => {}
        }
    }
}

fn update(
    state: Res<GameState>,
    player: Res<LocalPlayer>,
    mut ui: ResMut<WeaponUi>,
    mut status: Query<&mut Text, With<WeaponStatus>>,
    mut labels: Query<(&ActionLabel, &mut Text), Without<WeaponStatus>>,
    mut actions: Query<(&Action, &mut BackgroundColor)>,
) {
    let Some(weapons) = &state.weapons else {
        return;
    };
    let tube = &weapons.tubes[ui.tube];
    let tracks = state
        .tactical
        .as_ref()
        .map(|t| t.shared_tracks.as_slice())
        .unwrap_or(&[]);
    if !tracks.iter().any(|t| Some(t.id) == ui.target) {
        ui.target = tracks.first().map(|t| t.id);
        ui.confirmation = None;
    }
    if ui.confirmation.is_some_and(|(index, settings)| {
        index as usize != ui.tube
            || tube.settings != Some(settings)
            || !tube.authorized
            || tube.phase != TubePhase::Ready
    }) {
        ui.confirmation = None;
    }
    let solution = weapons.solutions[ui.tube]
        .as_ref()
        .map(|s| {
            format!(
                "Tir {:.0} deg / {:.0}s / +/- {:.0}m\n{}",
                s.bearing,
                s.run_seconds,
                s.uncertainty_meters,
                s.warnings.join(", ")
            )
        })
        .unwrap_or_else(|| "Solution indisponible : configurer une piste suivie".into());
    let field = match ui.field {
        0 => format!("Prof. {:.0}m", ui.depth),
        1 => format!("Cap cible {:+.0} deg", ui.heading),
        _ => format!("Vit. cible {:+.0} kn", ui.speed),
    };
    let feedback = state
        .last_error
        .as_ref()
        .map(|error| crate::render::error_label(error))
        .unwrap_or_else(|| format!("{} // accuse {:?}", ui.notice, state.last_accepted_command));
    let event = match &state.latest_event {
        Some(shared::GameEvent::TorpedoImpact { torpedo_id }) => {
            format!("Explosion observee / torpille {}", torpedo_id)
        }
        Some(shared::GameEvent::TorpedoExpired { torpedo_id }) => {
            format!("Fin de course / torpille {}", torpedo_id)
        }
        Some(shared::GameEvent::TorpedoFired { bearing }) => {
            format!("Torpille lancee / {:.0} deg", bearing)
        }
        _ => String::new(),
    };
    let selected = tracks
        .iter()
        .find(|t| Some(t.id) == ui.target)
        .map(|t| {
            format!(
                "P{} {:.0} deg / {:.0}m / conf. {:.0}%\nCible cap {:.0} / {:.1} kn",
                t.id.0,
                t.bearing,
                t.distance.unwrap_or(0.0),
                t.confidence,
                t.heading.unwrap_or(0.0),
                t.speed.unwrap_or(0.0)
            )
        })
        .unwrap_or_else(|| "Aucune piste partagee".into());
    for mut text in &mut status {
        **text = format!("{}\nTUBE {} {} // {} / {:.0}s\nReserve {} / mer {} / autorisation {}\nConfig. piste {:?} / prof. {:.0}m\n{}\n{}\n{}\nGlisser pour defiler",
            selected,
            ui.tube + 1, if ui.tube == 4 { "ARR" } else { "AV" }, phase_label(tube.phase), tube.remaining_seconds,
            weapons.reserve, weapons.torpedoes.len(), if tube.authorized { "OUI" } else { "NON" },
            tube.settings.map(|s| s.target.0), tube.settings.map_or(0.0, |s| s.depth), solution, feedback, event);
    }
    let captain = player.role == Some(CrewRole::Captain);
    for (ActionLabel(action), mut text) in &mut labels {
        **text = match action {
            Action::Target => format!(
                "Piste {} >",
                ui.target.map_or("---".into(), |id| id.0.to_string())
            ),
            Action::Tube => format!("Tube {} >", ui.tube + 1),
            Action::Field => field.clone(),
            Action::Minus => "−".into(),
            Action::Plus => "+".into(),
            Action::Configure => if captain { "Ordre bot" } else { "Configurer" }.into(),
            Action::Prepare => {
                if captain {
                    "Autoriser".into()
                } else {
                    match tube.phase {
                        TubePhase::Empty => "Charger",
                        TubePhase::Loaded => "Inonder",
                        TubePhase::Flooded if tube.settings.is_some() => "Ouvrir",
                        TubePhase::Flooded => "Purger",
                        TubePhase::Ready => "Pret",
                        _ => "En cours…",
                    }
                    .into()
                }
            }
            Action::Fire => if captain {
                "Tir : Armement"
            } else if ui.confirmation.is_some() {
                "CONFIRMER TIR"
            } else {
                "Tirer…"
            }
            .into(),
            Action::Cancel => if captain { "—" } else { "Fermer / purger" }.into(),
        };
    }
    for (action, mut color) in &mut actions {
        let enabled = match action {
            Action::Target => !tracks.is_empty(),
            Action::Tube | Action::Field => true,
            Action::Minus | Action::Plus => !captain,
            Action::Configure if captain => {
                ui.target.is_some()
                    && state.lobby.as_ref().is_some_and(|lobby| {
                        lobby.slots.iter().any(|slot| {
                            slot.role == CrewRole::Weapons
                                && slot.occupant == shared::RoleOccupant::Bot
                        })
                    })
            }
            Action::Configure => {
                ui.target.is_some()
                    && matches!(
                        tube.phase,
                        TubePhase::Loaded | TubePhase::Flooded | TubePhase::Ready
                    )
            }
            Action::Prepare if captain => {
                tube.settings.is_some() && weapons.solutions[ui.tube].is_some()
            }
            Action::Prepare => {
                matches!(tube.phase, TubePhase::Empty | TubePhase::Flooded)
                    || (tube.phase == TubePhase::Loaded && tube.settings.is_some())
            }
            Action::Fire => {
                !captain
                    && tube.phase == TubePhase::Ready
                    && tube.authorized
                    && weapons.solutions[ui.tube]
                        .as_ref()
                        .is_some_and(|s| s.launch_allowed)
            }
            Action::Cancel => {
                !captain && matches!(tube.phase, TubePhase::Ready | TubePhase::Flooded)
            }
        };
        *color = BackgroundColor(if enabled {
            Color::srgb(0.04, 0.18, 0.23)
        } else {
            Color::srgb(0.045, 0.065, 0.075)
        });
    }
}

fn phase_label(phase: TubePhase) -> &'static str {
    match phase {
        TubePhase::Empty => "Vide",
        TubePhase::Loading => "Chargement",
        TubePhase::Loaded => "Charge",
        TubePhase::Flooding => "Inondation",
        TubePhase::Flooded => "Inonde",
        TubePhase::Opening => "Ouverture",
        TubePhase::Ready => "Ouvert / pret",
        TubePhase::Closing => "Fermeture",
        TubePhase::Purging => "Purge",
    }
}
