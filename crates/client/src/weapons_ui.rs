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
        let Some(tube) = weapons.tubes.get(ui.tube) else {
            ui.notice = "Tube indisponible : attendre la synchronisation de l'armement.".into();
            continue;
        };
        match action {
            Action::Tube => {
                ui.tube = (ui.tube + 1) % weapons.tubes.len();
                if let Some(settings) = weapons.tubes.get(ui.tube).and_then(|tube| tube.settings) {
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
                ui.notice = if role == CrewRole::Captain {
                    format!(
                        "Ordre de preparation envoye pour P{}. Le bot choisit un tube disponible.",
                        target.0
                    )
                } else {
                    format!("Configuration envoyee : P{} vers tube {}. Attendre la confirmation du serveur.", target.0, ui.tube + 1)
                };
            }
            Action::Prepare if role == CrewRole::Captain => {
                let Some(settings) = tube.settings else {
                    ui.notice = "Autorisation impossible : ce tube n'a pas de configuration. L'Armement doit d'abord le configurer.".into();
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
                        ui.notice = tube_guidance(weapons, &tactical.shared_tracks, &ui, false);
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
                        ui.notice = format!("Etat du tube : {} ({:.0}s restantes). Fermer un tube ouvert, puis purger une fois la fermeture terminee.", phase_label(tube.phase), tube.remaining_seconds);
                        continue;
                    }
                };
                commands.push(command);
            }
            Action::Fire if role == CrewRole::Weapons => {
                let Some(settings) = tube.settings else {
                    ui.notice = "Tir impossible : aucune cible configuree dans ce tube. Choisir une piste puis Configurer.".into();
                    continue;
                };
                if tube.phase != TubePhase::Ready
                    || !tube.authorized
                    || pending_configuration(&ui, tube.settings)
                    || !weapons.solutions.get(ui.tube).and_then(|s| s.as_ref())
                        .is_some_and(|s| s.launch_allowed)
                {
                    ui.notice = tube_guidance(weapons, &tactical.shared_tracks, &ui, false);
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
    if weapons.tubes.is_empty() {
        ui.confirmation = None;
        for mut text in &mut status { **text = "Armement indisponible : aucun tube recu. Attendre la synchronisation.".into(); }
        for (_, mut color) in &mut actions { *color = BackgroundColor(Color::srgb(0.045, 0.065, 0.075)); }
        return;
    }
    if ui.tube >= weapons.tubes.len() { ui.tube = 0; ui.confirmation = None; }
    let Some(tube) = weapons.tubes.get(ui.tube) else { return; };
    let current_solution = weapons.solutions.get(ui.tube).and_then(|s| s.as_ref());
    let tracks = state
        .tactical
        .as_ref()
        .map(|t| t.shared_tracks.as_slice())
        .unwrap_or(&[]);
    if !tracks.iter().any(|t| Some(t.id) == ui.target) {
        if let Some(lost) = ui.target {
            ui.notice = format!("Piste selectionnee P{} perdue ou non partagee. La selection change automatiquement ; la configuration du tube reste inchangee.", lost.0);
        }
        ui.target = tracks.first().map(|t| t.id);
        ui.confirmation = None;
    }
    if ui.confirmation.is_some_and(|(index, settings)| {
        index as usize != ui.tube
            || tube.settings != Some(settings)
            || !tube.authorized
            || tube.phase != TubePhase::Ready
            || pending_configuration(&ui, tube.settings)
            || !current_solution
                .is_some_and(|s| s.launch_allowed)
    }) {
        ui.confirmation = None;
    }
    let solution = current_solution
        .map(|s| {
            format!(
                "Tir {:.0} deg / {:.0}s / +/- {:.0}m\n{}",
                s.bearing,
                s.run_seconds,
                s.uncertainty_meters,
                s.warnings.join(", ")
            )
        })
        .unwrap_or_else(|| "Aucune solution de tir pour ce tube.".into());
    let field = match ui.field {
        0 => format!("Prof. torpille {:.0}m", ui.depth),
        1 => format!("Correction cap cible {:+.0} deg", ui.heading),
        _ => format!("Correction vit. cible {:+.0} kn", ui.speed),
    };
    let feedback = state
        .last_error
        .as_ref()
        .map(|error| {
            format!(
                "Derniere commande refusee : {}",
                crate::render::error_label(error)
            )
        })
        .unwrap_or_else(|| {
            state.last_accepted_command.map_or_else(String::new, |id| {
                format!("Derniere commande acceptee : #{}.", id.0)
            })
        });
    let events = state.events.iter().rev().filter_map(|(tick, event)| match event {
        shared::GameEvent::TorpedoImpact { torpedo_id } => {
            Some(format!("[tick {}] Explosion observee / torpille {}", tick, torpedo_id))
        }
        shared::GameEvent::TorpedoExpired { torpedo_id } => {
            Some(format!("[tick {}] Fin de course estimee / torpille {} (impact non confirme)", tick, torpedo_id))
        }
        shared::GameEvent::TorpedoFired { bearing } => {
            Some(format!("[tick {}] Torpille lancee / {:.0} deg", tick, bearing))
        }
        _ => None,
    }).take(6).collect::<Vec<_>>();
    let event = events.into_iter().rev().collect::<Vec<_>>().join("\n");
    let selected = tracks
        .iter()
        .find(|t| Some(t.id) == ui.target)
        .map(|t| {
            format!(
                "SELECTION : P{} / {:.0} deg / {} / confiance piste {:.0}%\nCible estimee : cap {} / {}",
                t.id.0,
                t.bearing,
                t.distance.map_or("distance inconnue".into(), |v| format!("{v:.0}m")),
                t.confidence,
                t.heading.map_or("inconnu".into(), |v| format!("{v:.0} deg")),
                t.speed.map_or("vitesse inconnue".into(), |v| format!("{v:.1} kn"))
            )
        })
        .unwrap_or_else(|| "Aucune piste partagee".into());
    let guidance = tube_guidance(weapons, tracks, &ui, player.role == Some(CrewRole::Captain));
    let configured = tube.settings.map_or_else(|| "CONFIGURATION APPLIQUEE : aucune cible.".into(), |s|
        format!("CONFIGURATION APPLIQUEE : P{} / torpille {:.0}m / correction cap {:+.0} deg / vit. {:+.0} kn", s.target.0, s.depth, s.heading_correction, s.speed_correction));
    for mut text in &mut status {
        **text = format!("{}\nTUBE {} {} // {} / {:.0}s\nReserve {} / courses suivies {} / autorisation {}\n{}\n{}\n{}\n{}\nDerniere action locale : {}\n{}\nGlisser pour defiler",
            selected,
            ui.tube + 1, if ui.tube == 4 { "ARR" } else { "AV" }, phase_label(tube.phase), tube.remaining_seconds,
            weapons.reserve, weapons.torpedoes.len(), if tube.authorized { "OUI" } else { "NON" },
            configured, guidance, solution, feedback, ui.notice, event);
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
            Action::Minus => "-".into(),
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
                        TubePhase::Ready => "Tube ouvert",
                        _ => "Attendre...",
                    }
                    .into()
                }
            }
            Action::Fire => if captain {
                "Tir : Armement"
            } else if ui.confirmation.is_some() {
                "CONFIRMER TIR"
            } else {
                "Tirer..."
            }
            .into(),
            Action::Cancel => if captain {
                "Armement uniquement"
            } else {
                "Fermer / purger"
            }
            .into(),
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
                tube.settings.is_some() && current_solution.is_some()
            }
            Action::Prepare => {
                matches!(tube.phase, TubePhase::Empty | TubePhase::Flooded)
                    || (tube.phase == TubePhase::Loaded && tube.settings.is_some())
            }
            Action::Fire => {
                !captain
                    && !pending_configuration(&ui, tube.settings)
                    && tube.phase == TubePhase::Ready
                    && tube.authorized
                    && current_solution
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

fn pending_configuration(ui: &WeaponUi, applied: Option<ShotSettings>) -> bool {
    ui.target.map(|target| ShotSettings {
        target,
        depth: ui.depth,
        heading_correction: ui.heading,
        speed_correction: ui.speed,
    }) != applied
}

fn tube_guidance(
    weapons: &shared::WeaponsMeasurements,
    tracks: &[shared::TrackEstimate],
    ui: &WeaponUi,
    captain: bool,
) -> String {
    let Some(tube) = weapons.tubes.get(ui.tube) else {
        return "Tube indisponible : attendre la synchronisation de l'armement.".into();
    };
    let current_solution = weapons.solutions.get(ui.tube).and_then(|s| s.as_ref());
    let mut messages = Vec::new();
    if let Some(settings) = tube.settings {
        if !tracks.iter().any(|track| track.id == settings.target) {
            let recovery = if captain {
                "Demander a l'Armement de reconfigurer ce tube, ou donner un nouvel Ordre bot."
            } else {
                "Choisir une piste partagee puis Configurer."
            };
            messages.push(format!(
                "TIR BLOQUE : le tube {} vise P{}, piste perdue ou non partagee. {}",
                ui.tube + 1,
                settings.target.0,
                recovery
            ));
        }
        if !captain && ui.target != Some(settings.target) {
            messages.push(format!("SELECTION NON APPLIQUEE : le tube vise toujours P{}. Appuyer sur Configurer pour changer de cible.", settings.target.0));
        } else if !captain && pending_configuration(ui, tube.settings) {
            messages.push("REGLAGES NON APPLIQUES : appuyer sur Configurer avant de tirer (nouvelle autorisation necessaire).".into());
        }
        if let Some(error) = weapons
            .solution_errors
            .get(ui.tube)
            .and_then(|error| error.as_ref())
        {
            if !matches!(error, shared::ProtocolError::TrackNotShared) {
                messages.push(format!(
                    "SOLUTION BLOQUEE POUR P{} : {}",
                    settings.target.0,
                    crate::render::error_label(error)
                ));
            }
        }
    }
    let next = match tube.phase {
        TubePhase::Loading | TubePhase::Flooding | TubePhase::Opening | TubePhase::Closing | TubePhase::Purging =>
            format!("EN COURS : {}. Attendre {:.0}s ; ne pas relancer l'operation.", phase_label(tube.phase), tube.remaining_seconds.ceil()),
        TubePhase::Empty if weapons.reserve == 0 => "STOCK VIDE : aucune torpille en reserve. Verifier les autres tubes deja charges.".into(),
        TubePhase::Empty if captain => "PROCHAINE ETAPE : donner un Ordre bot ou demander a l'Armement de charger et configurer un tube.".into(),
        TubePhase::Empty => "PROCHAINE ETAPE : Charger, puis attendre la fin du chargement.".into(),
        TubePhase::Loaded if tube.settings.is_none() && captain => "PROCHAINE ETAPE : demander a l'Armement de configurer ce tube, ou donner un Ordre bot sur une piste partagee.".into(),
        TubePhase::Loaded if tube.settings.is_none() => "PROCHAINE ETAPE : choisir une piste et appuyer sur Configurer. Une torpille chargee ne suffit pas pour inonder.".into(),
        TubePhase::Loaded if captain => "PROCHAINE ETAPE : l'Armement doit Inonder puis Ouvrir le tube.".into(),
        TubePhase::Loaded => "PROCHAINE ETAPE : Inonder (8s), puis Ouvrir (3s).".into(),
        TubePhase::Flooded if tube.settings.is_none() => "PROCHAINE ETAPE : Purger avant de recharger.".into(),
        TubePhase::Flooded => "PROCHAINE ETAPE : Ouvrir pour poursuivre, ou Fermer / purger pour annuler la preparation.".into(),
        TubePhase::Ready if current_solution.is_none() => "Tube ouvert, mais aucune solution valide : resoudre le blocage indique ci-dessus, puis Configurer si necessaire.".into(),
        TubePhase::Ready if !current_solution.is_some_and(|s| s.launch_allowed) => "TIR BLOQUE PAR LE POSITIONNEMENT : consulter les avertissements de la solution et demander une manoeuvre au Pilote.".into(),
        TubePhase::Ready if !tube.authorized && captain => "AUTORISATION MANQUANTE : verifier la configuration appliquee puis appuyer sur Autoriser.".into(),
        TubePhase::Ready if !tube.authorized => "AUTORISATION MANQUANTE : demander au Capitaine d'autoriser ce tube (automatique si le Capitaine est un bot).".into(),
        TubePhase::Ready if captain => "TUBE PRET ET AUTORISE : l'Armement peut declencher le tir ; le bot le fait automatiquement.".into(),
        TubePhase::Ready if pending_configuration(ui, tube.settings) => "Tir suspendu : appliquer la selection et les reglages locaux avec Configurer, ou revenir aux reglages appliques.".into(),
        TubePhase::Ready => "PRET A TIRER : Tirer..., puis CONFIRMER TIR. Le tube ouvert n'a plus d'etape de preparation.".into(),
    };
    messages.push(next);
    messages.join("\n")
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
