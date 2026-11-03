use shared::{
    FiringSolution, GameEvent, PlayerCommand, ProtocolError, ShotSettings, SubmarineState,
    TorpedoMeasurements, TrackEstimate, TubeMeasurements, TubePhase, WeaponsConfig,
    WeaponsMeasurements,
};

use crate::{world::World, KNOTS_TO_METERS_PER_SECOND};

struct RunningTorpedo {
    measurement: TorpedoMeasurements,
    target_depth: f32,
    active: bool,
}

#[derive(Default)]
pub(crate) struct Weapons {
    tubes: Vec<TubeMeasurements>,
    reserve: u8,
    torpedoes: Vec<RunningTorpedo>,
    next_id: u64,
    bot_tube: Option<u8>,
    bot_settings: Option<ShotSettings>,
}

impl Weapons {
    pub(crate) fn new() -> Self {
        Self {
            tubes: (0..5)
                .map(|_| TubeMeasurements {
                    phase: TubePhase::Empty,
                    loaded: false,
                    remaining_seconds: 0.0,
                    settings: None,
                    authorized: false,
                })
                .collect(),
            reserve: 5,
            ..Self::default()
        }
    }

    pub(crate) fn measurements(
        &self,
        state: &SubmarineState,
        tracks: &[TrackEstimate],
        config: WeaponsConfig,
    ) -> WeaponsMeasurements {
        let results: Vec<_> = self
            .tubes
            .iter()
            .map(|tube| {
                tube.settings
                    .map(|settings| solution(state, tracks, settings, config))
            })
            .collect();
        WeaponsMeasurements {
            tubes: self.tubes.clone(),
            reserve: self.reserve,
            solution_errors: results
                .iter()
                .map(|result| result.as_ref().and_then(|r| r.as_ref().err().cloned()))
                .collect(),
            solutions: results
                .iter()
                .enumerate()
                .map(|(index, result)| {
                    result
                        .as_ref()
                        .and_then(|r| r.as_ref().ok())
                        .cloned()
                        .map(|mut shot| {
                            let axis = (state.heading + if index == 4 { 180.0 } else { 0.0 })
                                .rem_euclid(360.0);
                            if crate::shortest_angle(axis, shot.bearing).abs() > 90.0 {
                                shot.launch_allowed = false;
                                shot.warnings.push("Cible hors secteur du tube".into());
                            }
                            shot
                        })
                })
                .collect(),
            torpedoes: self
                .torpedoes
                .iter()
                .map(|t| t.measurement.clone())
                .collect(),
        }
    }

    pub(crate) fn apply(
        &mut self,
        command: PlayerCommand,
        state: &SubmarineState,
        tracks: &[TrackEstimate],
        config: WeaponsConfig,
    ) -> Result<Vec<GameEvent>, ProtocolError> {
        use PlayerCommand::*;
        use TubePhase::*;
        if let OrderWeaponsBot { target } = command {
            solution(state, tracks, ShotSettings::for_target(target), config)?;
            // Un nouvel ordre remplace la consigne du tube deja pris en charge.
            // La configuration sera appliquee par la meme commande que pour un humain,
            // une fois la phase de chargement terminee.
            let index = self
                .bot_tube
                .map(|index| index as usize)
                .or_else(|| {
                    self.tubes
                        .iter()
                        .position(|t| t.phase == Loaded)
                        .or_else(|| self.tubes.iter().position(|t| t.phase == Empty))
                })
                .ok_or(ProtocolError::InvalidTubePhase)?;
            let settings = ShotSettings::for_target(target);
            if !self.tubes[index].loaded && !matches!(self.tubes[index].phase, Empty | Loading) {
                return Err(ProtocolError::InvalidTubePhase);
            }
            if self.tubes[index].phase == Empty {
                self.apply(LoadTube(index as u8), state, tracks, config)?;
            }
            self.tubes[index].authorized = false;
            self.bot_tube = Some(index as u8);
            self.bot_settings = Some(settings);
            return Ok(vec![]);
        }
        let index = match command {
            LoadTube(t) | FloodTube(t) | OpenTube(t) | CloseTube(t) | PurgeTube(t) => t,
            ConfigureShot { tube, .. } | AuthorizeShot { tube, .. } | FireTorpedo { tube } => tube,
            _ => return Ok(vec![]),
        };
        let tube = self
            .tubes
            .get_mut(index as usize)
            .ok_or(ProtocolError::InvalidTube)?;
        match command {
            LoadTube(_) if tube.phase == Empty => {
                if self.reserve == 0 {
                    return Err(ProtocolError::NoTorpedoes);
                }
                self.reserve -= 1;
                tube.phase = Loading;
                tube.remaining_seconds = config.load_seconds;
                tube.settings = None;
                tube.authorized = false;
            }
            ConfigureShot { settings, .. }
                if tube.loaded && matches!(tube.phase, Loaded | Flooded | Ready) =>
            {
                validate_settings(settings, config)?;
                solution(state, tracks, settings, config)?;
                tube.settings = Some(settings);
                tube.authorized = false;
            }
            FloodTube(_) if tube.phase == Loaded && tube.settings.is_some() => {
                tube.phase = Flooding;
                tube.remaining_seconds = config.flood_seconds;
            }
            OpenTube(_) if tube.phase == Flooded => {
                tube.phase = Opening;
                tube.remaining_seconds = config.door_seconds;
            }
            CloseTube(_) if tube.phase == Ready => {
                tube.phase = Closing;
                tube.remaining_seconds = config.door_seconds;
                tube.authorized = false;
            }
            PurgeTube(_) if tube.phase == Flooded => {
                tube.phase = Purging;
                tube.remaining_seconds = config.purge_seconds;
                tube.authorized = false;
            }
            AuthorizeShot { settings, .. } => {
                if tube.settings != Some(settings) {
                    return Err(ProtocolError::AuthorizationSettingsMismatch);
                }
                if !tube.loaded {
                    return Err(ProtocolError::InvalidTubePhase);
                }
                validate_settings(settings, config)?;
                solution(state, tracks, settings, config)?;
                tube.authorized = true;
            }
            FireTorpedo { .. } if tube.phase == Ready && tube.loaded => {
                if !tube.authorized {
                    return Err(ProtocolError::ShotNotAuthorized);
                }
                if state.depth > config.max_launch_depth {
                    return Err(ProtocolError::LaunchDepthExceeded);
                }
                let settings = tube.settings.ok_or(ProtocolError::InvalidShotSettings)?;
                let shot = solution(state, tracks, settings, config)?;
                // Tubes avant et arrière : le bâtiment doit être positionné pour le tir.
                let axis = (state.heading + if index == 4 { 180.0 } else { 0.0 }).rem_euclid(360.0);
                if crate::shortest_angle(axis, shot.bearing).abs() > 90.0 {
                    return Err(ProtocolError::TargetOutsideTubeSector);
                }
                self.next_id += 1;
                self.torpedoes.push(RunningTorpedo {
                    measurement: TorpedoMeasurements {
                        id: self.next_id,
                        x: state.x,
                        y: state.y,
                        depth: state.depth,
                        bearing: shot.bearing,
                        remaining_meters: config.torpedo_range_meters,
                    },
                    target_depth: settings.depth,
                    active: true,
                });
                tube.phase = Closing;
                tube.remaining_seconds = config.door_seconds;
                tube.settings = None;
                tube.loaded = false;
                tube.authorized = false;
                return Ok(vec![GameEvent::TorpedoFired {
                    bearing: shot.bearing,
                }]);
            }
            _ => return Err(ProtocolError::InvalidTubePhase),
        }
        Ok(vec![])
    }

    pub(crate) fn automate(
        &mut self,
        state: &SubmarineState,
        tracks: &[TrackEstimate],
        config: WeaponsConfig,
        weapons_bot: bool,
        captain_bot: bool,
    ) -> Vec<GameEvent> {
        if captain_bot {
            for tube in &mut self.tubes {
                if let Some(settings) = tube.settings {
                    tube.authorized = solution(state, tracks, settings, config).is_ok();
                }
            }
        }
        if !weapons_bot {
            return vec![];
        }
        let Some(index) = self.bot_tube else {
            return vec![];
        };
        let tube = &self.tubes[index as usize];
        let pending = self.bot_settings;
        if pending.is_some_and(|settings| !tracks.iter().any(|track| track.id == settings.target)) {
            // Recuperer le tube apres la perte d'une piste au lieu de bloquer
            // indefiniment le bot. Une munition non tiree reste chargee.
            let cleanup = match tube.phase {
                TubePhase::Ready => Some(PlayerCommand::CloseTube(index)),
                TubePhase::Flooded => Some(PlayerCommand::PurgeTube(index)),
                TubePhase::Loaded | TubePhase::Empty => {
                    self.bot_tube = None;
                    self.bot_settings = None;
                    return vec![];
                }
                _ => None,
            };
            return cleanup
                .and_then(|command| self.apply(command, state, tracks, config).ok())
                .unwrap_or_default();
        }
        let command = match tube.phase {
            TubePhase::Loaded | TubePhase::Flooded | TubePhase::Ready
                if tube.loaded && pending.is_some() && tube.settings != pending =>
            {
                Some(PlayerCommand::ConfigureShot {
                    tube: index,
                    settings: pending.unwrap(),
                })
            }
            TubePhase::Loaded => Some(PlayerCommand::FloodTube(index)),
            TubePhase::Flooded if tube.loaded => Some(PlayerCommand::OpenTube(index)),
            TubePhase::Flooded => Some(PlayerCommand::PurgeTube(index)),
            TubePhase::Ready if tube.authorized => Some(PlayerCommand::FireTorpedo { tube: index }),
            TubePhase::Empty => {
                self.bot_tube = None;
                self.bot_settings = None;
                None
            }
            _ => None,
        };
        let fired = matches!(command, Some(PlayerCommand::FireTorpedo { .. }));
        match command.map(|c| self.apply(c, state, tracks, config)) {
            Some(Ok(events)) => {
                // La fermeture et la purge restent a la charge du bot, mais la
                // consigne consommee ne doit pas reconfigurer un tube vide.
                if fired {
                    self.bot_settings = None;
                }
                events
            }
            _ => vec![],
        }
    }

    pub(crate) fn update(
        &mut self,
        dt: f32,
        world: &mut World,
        config: WeaponsConfig,
        state: &SubmarineState,
    ) -> Vec<GameEvent> {
        for tube in &mut self.tubes {
            tube.remaining_seconds = (tube.remaining_seconds - dt).max(0.0);
            if tube.remaining_seconds == 0.0 {
                tube.phase = match tube.phase {
                    TubePhase::Loading => {
                        tube.loaded = true;
                        TubePhase::Loaded
                    }
                    TubePhase::Flooding => TubePhase::Flooded,
                    TubePhase::Opening => TubePhase::Ready,
                    TubePhase::Closing => TubePhase::Flooded,
                    TubePhase::Purging if tube.loaded => TubePhase::Loaded,
                    TubePhase::Purging => TubePhase::Empty,
                    phase => phase,
                };
            }
        }
        let mut events = vec![];
        self.torpedoes.retain_mut(|run| {
            let torpedo = &mut run.measurement;
            let distance = (config.torpedo_speed_knots * KNOTS_TO_METERS_PER_SECOND * dt)
                .min(torpedo.remaining_meters);
            let angle = torpedo.bearing.to_radians();
            let (old_x, old_y) = (torpedo.x, torpedo.y);
            torpedo.x += angle.sin() * distance;
            torpedo.y += angle.cos() * distance;
            torpedo.remaining_meters -= distance;
            torpedo.depth += (run.target_depth - torpedo.depth).clamp(-dt * 1.5, dt * 1.5);
            if run.active
                && world
                    .impact(old_x, old_y, torpedo.x, torpedo.y, torpedo.depth, config)
                    .is_some()
            {
                run.active = false;
                if (torpedo.x - state.x).hypot(torpedo.y - state.y) <= 6000.0 {
                    events.push(GameEvent::ExplosionHeard);
                    events.push(GameEvent::TorpedoImpact {
                        torpedo_id: torpedo.id,
                    });
                    return false;
                }
                // Au-delà de l'écoute, conserver la course estimée : ne pas révéler l'impact caché.
            }
            if torpedo.remaining_meters <= 0.0 {
                events.push(GameEvent::TorpedoExpired {
                    torpedo_id: torpedo.id,
                });
                return false;
            }
            true
        });
        events
    }

    pub(crate) fn procedure_noise(&self) -> f32 {
        self.tubes
            .iter()
            .filter(|t| {
                matches!(
                    t.phase,
                    TubePhase::Loading
                        | TubePhase::Flooding
                        | TubePhase::Opening
                        | TubePhase::Closing
                        | TubePhase::Purging
                )
            })
            .count() as f32
            * 8.0
    }

    pub(crate) fn needs_bot_order(&self) -> bool {
        self.bot_tube.is_none()
            && (self.reserve > 0
                || self
                    .tubes
                    .iter()
                    .any(|tube| tube.phase == TubePhase::Loaded))
    }
}

fn validate_settings(settings: ShotSettings, config: WeaponsConfig) -> Result<(), ProtocolError> {
    if !settings.depth.is_finite()
        || !(1.0..=15.0).contains(&settings.depth)
        || !settings.heading_correction.is_finite()
        || settings.heading_correction.abs() > 45.0
        || !settings.speed_correction.is_finite()
        || settings.speed_correction.abs() > 10.0
    {
        return Err(ProtocolError::InvalidShotSettings);
    }
    if settings.depth > config.max_surface_impact_depth {
        return Err(ProtocolError::TorpedoDepthExceedsImpactZone {
            maximum_meters: config.max_surface_impact_depth,
        });
    }
    Ok(())
}

pub(crate) fn solution(
    state: &SubmarineState,
    tracks: &[TrackEstimate],
    settings: ShotSettings,
    config: WeaponsConfig,
) -> Result<FiringSolution, ProtocolError> {
    validate_settings(settings, config)?;
    let track = tracks
        .iter()
        .find(|t| t.id == settings.target)
        .ok_or(ProtocolError::TrackNotShared)?;
    if !track.shared {
        return Err(ProtocolError::TrackNotShared);
    }
    let distance = track.distance.ok_or(ProtocolError::MissingTargetDistance)?;
    let heading = (track.heading.ok_or(ProtocolError::MissingTargetHeading)?
        + settings.heading_correction)
        .to_radians();
    let speed = (track.speed.ok_or(ProtocolError::MissingTargetSpeed)? + settings.speed_correction)
        .max(0.0)
        * KNOTS_TO_METERS_PER_SECOND;
    let angle = track.bearing.to_radians();
    let (rx, ry) = (angle.sin() * distance, angle.cos() * distance);
    let (vx, vy) = (heading.sin() * speed, heading.cos() * speed);
    let ts = config.torpedo_speed_knots * KNOTS_TO_METERS_PER_SECOND;
    let a = vx * vx + vy * vy - ts * ts;
    let b = 2.0 * (rx * vx + ry * vy);
    let c = rx * rx + ry * ry;
    let discriminant = b * b - 4.0 * a * c;
    // Quand les vitesses sont proches, l'equation devient lineaire : b*t+c=0.
    // Ne pas diviser par a, ce qui amplifie les erreurs d'arrondi.
    let time = if a.abs() <= ts * ts * 1e-6 {
        let time = -c / b;
        if !time.is_finite() || time <= 0.0 {
            return Err(ProtocolError::NoFiringSolution);
        }
        time
    } else {
        if discriminant < 0.0 {
            return Err(ProtocolError::NoFiringSolution);
        }
        let roots = [
            (-b + discriminant.sqrt()) / (2.0 * a),
            (-b - discriminant.sqrt()) / (2.0 * a),
        ];
        roots
            .into_iter()
            .filter(|t| t.is_finite() && *t > 0.0)
            .reduce(f32::min)
            .ok_or(ProtocolError::NoFiringSolution)?
    };
    if time * ts > config.torpedo_range_meters {
        return Err(ProtocolError::InterceptOutOfRange);
    }
    let bearing = (rx + vx * time)
        .atan2(ry + vy * time)
        .to_degrees()
        .rem_euclid(360.0);
    let mut warnings = vec![];
    if track.confidence < 70.0 {
        warnings.push("Piste peu fiable".into());
    }
    if track.distance_uncertainty.unwrap_or(distance) > 300.0 {
        warnings.push("Distance incertaine".into());
    }
    if track.bearing_uncertainty > 2.0 {
        warnings.push("Relevement incertain".into());
    }
    if state.depth > config.max_launch_depth {
        warnings.push("Trop profond pour lancer".into());
    }
    Ok(FiringSolution {
        bearing,
        run_seconds: time,
        uncertainty_meters: track.distance_uncertainty.unwrap_or(distance)
            + distance * track.bearing_uncertainty.to_radians().sin().abs(),
        warnings,
        launch_allowed: state.depth <= config.max_launch_depth,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{ContactClassification, TrackId};

    fn track() -> TrackEstimate {
        TrackEstimate {
            id: TrackId(42),
            bearing: 0.0,
            bearing_uncertainty: 0.5,
            distance: Some(1000.0),
            distance_uncertainty: Some(20.0),
            heading: Some(90.0),
            speed: Some(8.0),
            classification: ContactClassification::Merchant,
            confidence: 90.0,
            last_observation_tick: 0,
            shared: true,
        }
    }

    fn ready() -> (
        Weapons,
        World,
        SubmarineState,
        WeaponsConfig,
        Vec<TrackEstimate>,
    ) {
        let mut weapons = Weapons::new();
        let mut world = World::new(0);
        let state = SubmarineState::default();
        let config = WeaponsConfig::default();
        let tracks = vec![track()];
        let settings = ShotSettings::for_target(TrackId(42));
        for (command, wait) in [
            (PlayerCommand::LoadTube(0), config.load_seconds),
            (PlayerCommand::ConfigureShot { tube: 0, settings }, 0.0),
            (PlayerCommand::FloodTube(0), config.flood_seconds),
            (PlayerCommand::OpenTube(0), config.door_seconds),
        ] {
            weapons.apply(command, &state, &tracks, config).unwrap();
            weapons.update(wait, &mut world, config, &state);
        }
        (weapons, world, state, config, tracks)
    }

    #[test]
    fn preparation_stock_and_authorization_are_enforced() {
        let mut weapons = Weapons::new();
        let state = SubmarineState::default();
        let config = WeaponsConfig::default();
        assert_eq!(
            weapons.apply(
                PlayerCommand::FireTorpedo { tube: 0 },
                &state,
                &[track()],
                config
            ),
            Err(ProtocolError::InvalidTubePhase)
        );
        assert_eq!(
            weapons.apply(PlayerCommand::OpenTube(0), &state, &[track()], config),
            Err(ProtocolError::InvalidTubePhase)
        );
        for tube in 0..5 {
            weapons
                .apply(PlayerCommand::LoadTube(tube), &state, &[track()], config)
                .unwrap();
        }
        assert_eq!(weapons.reserve, 0);
        assert_eq!(
            weapons
                .tubes
                .iter()
                .filter(|t| t.phase == TubePhase::Loading)
                .count(),
            5
        );
        assert_eq!(
            weapons.apply(PlayerCommand::LoadTube(5), &state, &[track()], config),
            Err(ProtocolError::InvalidTube)
        );
        let (mut weapons, mut world, mut state, config, tracks) = ready();
        assert_eq!(
            weapons.apply(
                PlayerCommand::FireTorpedo { tube: 0 },
                &state,
                &tracks,
                config
            ),
            Err(ProtocolError::ShotNotAuthorized)
        );
        let settings = ShotSettings::for_target(TrackId(42));
        weapons
            .apply(
                PlayerCommand::AuthorizeShot { tube: 0, settings },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        state.depth = 30.0;
        assert_eq!(
            weapons.apply(
                PlayerCommand::FireTorpedo { tube: 0 },
                &state,
                &tracks,
                config
            ),
            Err(ProtocolError::LaunchDepthExceeded)
        );
        state.depth = 0.0;
        weapons
            .apply(
                PlayerCommand::FireTorpedo { tube: 0 },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        assert_eq!(weapons.reserve, 4);
        assert_eq!(weapons.torpedoes.len(), 1);
        assert_eq!(
            weapons.apply(
                PlayerCommand::FireTorpedo { tube: 0 },
                &state,
                &tracks,
                config
            ),
            Err(ProtocolError::InvalidTubePhase)
        );
        weapons.update(config.door_seconds, &mut world, config, &state);
        weapons
            .apply(PlayerCommand::PurgeTube(0), &state, &tracks, config)
            .unwrap();
        weapons.update(config.purge_seconds, &mut world, config, &state);
        weapons.reserve = 0;
        assert_eq!(
            weapons.apply(PlayerCommand::LoadTube(0), &state, &tracks, config),
            Err(ProtocolError::NoTorpedoes)
        );
    }

    #[test]
    fn new_configuration_invalidates_authorization_and_rejects_nonfinite_values() {
        let (mut weapons, _, state, config, tracks) = ready();
        let mut settings = ShotSettings::for_target(TrackId(42));
        weapons
            .apply(
                PlayerCommand::AuthorizeShot { tube: 0, settings },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        settings.speed_correction = 2.0;
        weapons
            .apply(
                PlayerCommand::ConfigureShot { tube: 0, settings },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        assert!(!weapons.tubes[0].authorized);
        settings.depth = f32::NAN;
        assert_eq!(
            weapons.apply(
                PlayerCommand::ConfigureShot { tube: 0, settings },
                &state,
                &tracks,
                config
            ),
            Err(ProtocolError::InvalidShotSettings)
        );
        assert_eq!(
            weapons.apply(
                PlayerCommand::AuthorizeShot {
                    tube: 0,
                    settings: weapons.tubes[0].settings.unwrap()
                },
                &state,
                &[],
                config
            ),
            Err(ProtocolError::TrackNotShared)
        );
    }

    #[test]
    fn solution_uses_only_the_observed_estimate_and_exposes_uncertainty() {
        let state = SubmarineState::default();
        let config = WeaponsConfig::default();
        let settings = ShotSettings::for_target(TrackId(42));
        let first = solution(&state, &[track()], settings, config).unwrap();
        let mut poor = track();
        poor.distance = Some(1500.0);
        poor.distance_uncertainty = Some(600.0);
        poor.bearing_uncertainty = 8.0;
        poor.confidence = 30.0;
        let second = solution(&state, &[poor], settings, config).unwrap();
        assert!(second.run_seconds > first.run_seconds);
        assert!(second.uncertainty_meters > first.uncertainty_meters);
        assert!(second.warnings.len() > first.warnings.len());
    }

    fn attack(change_route: bool, bearing_error: f32) -> (Vec<GameEvent>, World) {
        let (mut weapons, mut world, state, config, mut tracks) = ready();
        world.vessels[0].x = 0.0;
        world.vessels[0].y = 1000.0;
        world.vessels[0].heading = 90.0;
        world.vessels[0].speed = 8.0;
        world.vessels[1].x = 6000.0;
        world.vessels[2].x = -6000.0;
        tracks[0].bearing = bearing_error;
        let settings = weapons.tubes[0].settings.unwrap();
        weapons
            .apply(
                PlayerCommand::AuthorizeShot { tube: 0, settings },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        weapons
            .apply(
                PlayerCommand::FireTorpedo { tube: 0 },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        if change_route {
            world.vessels[0].heading = 270.0;
        }
        let mut events = vec![];
        for _ in 0..9000 {
            world.update(0.05, &state, 0, shared::EscortConfig::default());
            events.extend(weapons.update(0.05, &mut world, config, &state));
        }
        (events, world)
    }

    #[test]
    fn simulated_interception_hits_but_changed_route_or_bad_bearing_misses() {
        let (events, world) = attack(false, 0.0);
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::TorpedoImpact { .. })));
        assert!(!world.vessels[0].afloat());
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, GameEvent::ExplosionHeard))
                .count(),
            1
        );
        for (changed, error) in [(true, 0.0), (false, 15.0)] {
            let (events, world) = attack(changed, error);
            assert!(world.vessels[0].afloat());
            assert!(!events.contains(&GameEvent::ExplosionHeard));
            assert!(!events
                .iter()
                .any(|e| matches!(e, GameEvent::TorpedoImpact { .. })));
            assert!(events
                .iter()
                .any(|e| matches!(e, GameEvent::TorpedoExpired { .. })));
        }
    }

    #[test]
    fn bot_prepares_only_on_order_and_waits_for_captain() {
        let mut weapons = Weapons::new();
        let mut world = World::new(0);
        let state = SubmarineState::default();
        let config = WeaponsConfig::default();
        let tracks = [track()];
        weapons.automate(&state, &tracks, config, true, false);
        assert_eq!(weapons.reserve, 5);
        weapons
            .apply(
                PlayerCommand::OrderWeaponsBot {
                    target: TrackId(42),
                },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        for _ in 0..80 {
            weapons.update(1.0, &mut world, config, &state);
            weapons.automate(&state, &tracks, config, true, false);
        }
        assert!(weapons.torpedoes.is_empty());
        assert_eq!(weapons.tubes[0].phase, TubePhase::Ready);
        assert_eq!(
            weapons.automate(&state, &tracks, config, true, true).len(),
            1
        );
        assert_eq!(weapons.torpedoes.len(), 1);
    }

    #[test]
    fn attack_replay_is_deterministic() {
        assert_eq!(attack(false, 0.0), attack(false, 0.0));
    }

    #[test]
    fn solution_failures_distinguish_missing_motion_range_and_lost_track() {
        let (weapons, _, state, config, tracks) = ready();
        let settings = weapons.tubes[0].settings.unwrap();
        for (missing, expected) in [
            (0, ProtocolError::MissingTargetDistance),
            (1, ProtocolError::MissingTargetHeading),
            (2, ProtocolError::MissingTargetSpeed),
        ] {
            let mut estimate = tracks[0].clone();
            match missing {
                0 => estimate.distance = None,
                1 => estimate.heading = None,
                _ => estimate.speed = None,
            }
            assert_eq!(
                solution(&state, &[estimate.clone()], settings, config),
                Err(expected.clone())
            );
            assert_eq!(
                weapons
                    .measurements(&state, &[estimate], config)
                    .solution_errors[0],
                Some(expected)
            );
        }
        let mut fast = tracks[0].clone();
        fast.speed = Some(config.torpedo_speed_knots);
        fast.heading = Some(0.0);
        assert_eq!(
            solution(&state, &[fast], settings, config),
            Err(ProtocolError::NoFiringSolution)
        );
        let short_range = WeaponsConfig {
            torpedo_range_meters: 100.0,
            ..config
        };
        assert_eq!(
            solution(&state, &tracks, settings, short_range),
            Err(ProtocolError::InterceptOutOfRange)
        );
        let lost = weapons.measurements(&state, &[], config);
        assert_eq!(lost.solution_errors[0], Some(ProtocolError::TrackNotShared));
        assert!(lost.solutions[0].is_none());
        assert_eq!(lost.tubes[0].settings.unwrap().target, settings.target);
        let encoded = shared::codec::encode(&lost);
        assert_eq!(
            shared::codec::decode::<WeaponsMeasurements>(&encoded).unwrap(),
            lost
        );
    }

    #[test]
    fn a_valid_solution_outside_the_tube_sector_is_distinct_from_no_interception() {
        let (mut weapons, _, mut state, config, tracks) = ready();
        state.heading = 180.0;
        let settings = weapons.tubes[0].settings.unwrap();
        weapons
            .apply(
                PlayerCommand::AuthorizeShot { tube: 0, settings },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        let projection = weapons.measurements(&state, &tracks, config);
        assert!(projection.solution_errors[0].is_none());
        assert!(!projection.solutions[0].as_ref().unwrap().launch_allowed);
        assert_eq!(
            weapons.apply(
                PlayerCommand::FireTorpedo { tube: 0 },
                &state,
                &tracks,
                config
            ),
            Err(ProtocolError::TargetOutsideTubeSector)
        );
    }

    #[test]
    fn an_unheard_impact_does_not_reveal_the_hidden_sink_or_end_the_estimated_run() {
        let (mut weapons, mut world, state, mut config, mut tracks) = ready();
        config.torpedo_range_meters = 12000.0;
        tracks[0].distance = Some(7000.0);
        tracks[0].speed = Some(0.0);
        world.vessels[0].x = 0.0;
        world.vessels[0].y = 7000.0;
        world.vessels[0].speed = 0.0;
        world.vessels[1].x = 6000.0;
        world.vessels[2].x = -6000.0;
        let settings = weapons.tubes[0].settings.unwrap();
        weapons
            .apply(
                PlayerCommand::AuthorizeShot { tube: 0, settings },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        weapons
            .apply(
                PlayerCommand::FireTorpedo { tube: 0 },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        for _ in 0..7200 {
            assert!(weapons.update(0.05, &mut world, config, &state).is_empty());
        }
        assert!(!world.vessels[0].afloat());
        let projection = weapons.measurements(&state, &tracks, config);
        assert_eq!(projection.torpedoes.len(), 1);
        assert!(projection.torpedoes[0].remaining_meters > 4000.0);
    }

    #[test]
    fn damage_is_configurable_and_collision_checks_the_whole_segment() {
        let mut world = World::new(0);
        world.vessels[0].x = 0.0;
        world.vessels[0].y = 1000.0;
        let config = WeaponsConfig {
            damage: 50.0,
            ..WeaponsConfig::default()
        };
        assert_eq!(world.impact(0.0, 0.0, 0.0, 2000.0, 12.0, config), None);
        assert_eq!(
            world.impact(0.0, 0.0, 0.0, 2000.0, 4.0, config),
            Some(false)
        );
        assert!(world.vessels[0].afloat());
        assert_eq!(world.impact(0.0, 0.0, 0.0, 2000.0, 4.0, config), Some(true));
        assert!(!world.vessels[0].afloat());
        assert!(
            crate::detection::active_observations(&world, &SubmarineState::default(), 0, 0, &mut 0)
                .len()
                == 2
        );
    }

    #[test]
    fn surface_depth_limit_is_shared_by_configuration_and_collisions() {
        let (mut weapons, mut world, state, config, tracks) = ready();
        let mut settings = weapons.tubes[0].settings.unwrap();
        settings.depth = 9.0;
        let before = weapons.measurements(&state, &tracks, config);
        assert_eq!(
            weapons.apply(
                PlayerCommand::ConfigureShot { tube: 0, settings },
                &state,
                &tracks,
                config
            ),
            Err(ProtocolError::TorpedoDepthExceedsImpactZone {
                maximum_meters: 8.0
            })
        );
        assert_eq!(weapons.measurements(&state, &tracks, config), before);
        let deeper_config = WeaponsConfig {
            max_surface_impact_depth: 10.0,
            ..config
        };
        weapons
            .apply(
                PlayerCommand::ConfigureShot { tube: 0, settings },
                &state,
                &tracks,
                deeper_config,
            )
            .unwrap();
        world.vessels[0].x = 0.0;
        world.vessels[0].y = 1000.0;
        assert_eq!(world.impact(0.0, 0.0, 0.0, 2000.0, 9.0, config), None);
        assert_eq!(
            world.impact(0.0, 0.0, 0.0, 2000.0, 9.0, deeper_config),
            Some(true)
        );
    }

    #[test]
    fn stale_authorization_reports_configuration_mismatch_without_changing_the_tube() {
        let (mut weapons, _, state, config, tracks) = ready();
        let old = weapons.tubes[0].settings.unwrap();
        let changed = ShotSettings { depth: 3.0, ..old };
        weapons
            .apply(
                PlayerCommand::ConfigureShot {
                    tube: 0,
                    settings: changed,
                },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        assert_eq!(
            weapons.apply(
                PlayerCommand::AuthorizeShot {
                    tube: 0,
                    settings: old
                },
                &state,
                &tracks,
                config
            ),
            Err(ProtocolError::AuthorizationSettingsMismatch)
        );
        assert!(!weapons.tubes[0].authorized);
        assert_eq!(weapons.tubes[0].settings, Some(changed));
    }

    #[test]
    fn repeated_bot_orders_retarget_one_tube_and_configure_only_after_loading() {
        let mut weapons = Weapons::new();
        let mut world = World::new(0);
        let state = SubmarineState::default();
        let config = WeaponsConfig::default();
        let first = track();
        let second = TrackEstimate {
            id: TrackId(43),
            ..track()
        };
        let tracks = [first, second];
        for target in [TrackId(42), TrackId(43)] {
            weapons
                .apply(
                    PlayerCommand::OrderWeaponsBot { target },
                    &state,
                    &tracks,
                    config,
                )
                .unwrap();
        }
        assert_eq!(weapons.reserve, 4);
        assert_eq!(weapons.tubes[0].phase, TubePhase::Loading);
        assert!(weapons.tubes[0].settings.is_none());
        assert!(weapons.tubes[1..]
            .iter()
            .all(|t| t.phase == TubePhase::Empty));
        let mut events = vec![];
        for _ in 0..80 {
            weapons.update(1.0, &mut world, config, &state);
            events.extend(weapons.automate(&state, &tracks, config, true, false));
        }
        assert_eq!(weapons.tubes[0].settings.unwrap().target, TrackId(43));
        assert_eq!(weapons.tubes[0].phase, TubePhase::Ready);
        assert!(events.is_empty());
        let settings = weapons.tubes[0].settings.unwrap();
        weapons
            .apply(
                PlayerCommand::AuthorizeShot { tube: 0, settings },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        assert_eq!(
            weapons.automate(&state, &tracks, config, true, false).len(),
            1
        );
        for _ in 0..20 {
            weapons.update(1.0, &mut world, config, &state);
            weapons.automate(&state, &tracks, config, true, false);
        }
        assert_eq!(weapons.tubes[0].phase, TubePhase::Empty);
        assert_eq!(weapons.reserve, 4);
    }

    #[test]
    fn bot_recovers_a_lost_track_without_discarding_the_loaded_torpedo() {
        let mut weapons = Weapons::new();
        let mut world = World::new(0);
        let state = SubmarineState::default();
        let config = WeaponsConfig::default();
        let tracks = [track()];
        weapons
            .apply(
                PlayerCommand::OrderWeaponsBot {
                    target: TrackId(42),
                },
                &state,
                &tracks,
                config,
            )
            .unwrap();
        for _ in 0..40 {
            weapons.update(1.0, &mut world, config, &state);
            weapons.automate(&state, &tracks, config, true, false);
        }
        assert_eq!(weapons.tubes[0].phase, TubePhase::Ready);
        for _ in 0..20 {
            weapons.update(1.0, &mut world, config, &state);
            weapons.automate(&state, &[], config, true, false);
        }
        assert_eq!(weapons.tubes[0].phase, TubePhase::Loaded);
        assert!(weapons.tubes[0].loaded);
        assert!(weapons.needs_bot_order());
        assert_eq!(weapons.reserve, 4);
        let new_tracks = [TrackEstimate {
            id: TrackId(43),
            ..track()
        }];
        weapons
            .apply(
                PlayerCommand::OrderWeaponsBot {
                    target: TrackId(43),
                },
                &state,
                &new_tracks,
                config,
            )
            .unwrap();
        weapons.automate(&state, &new_tracks, config, true, false);
        assert_eq!(weapons.tubes[0].settings.unwrap().target, TrackId(43));
        assert_eq!(weapons.reserve, 4);
    }

    #[test]
    fn purging_keeps_unfired_ammunition_but_cannot_recreate_a_fired_torpedo() {
        let (mut weapons, mut world, state, config, tracks) = ready();
        weapons
            .apply(PlayerCommand::CloseTube(0), &state, &tracks, config)
            .unwrap();
        weapons.update(config.door_seconds, &mut world, config, &state);
        weapons
            .apply(PlayerCommand::PurgeTube(0), &state, &tracks, config)
            .unwrap();
        weapons.update(config.purge_seconds, &mut world, config, &state);
        assert_eq!(weapons.tubes[0].phase, TubePhase::Loaded);
        assert!(weapons.tubes[0].loaded);
        let settings = weapons.tubes[0].settings.unwrap();
        for (command, wait) in [
            (PlayerCommand::FloodTube(0), config.flood_seconds),
            (PlayerCommand::OpenTube(0), config.door_seconds),
            (PlayerCommand::AuthorizeShot { tube: 0, settings }, 0.0),
            (PlayerCommand::FireTorpedo { tube: 0 }, config.door_seconds),
        ] {
            weapons.apply(command, &state, &tracks, config).unwrap();
            weapons.update(wait, &mut world, config, &state);
        }
        assert!(!weapons.tubes[0].loaded);
        assert_eq!(
            weapons.apply(
                PlayerCommand::ConfigureShot { tube: 0, settings },
                &state,
                &tracks,
                config
            ),
            Err(ProtocolError::InvalidTubePhase)
        );
        weapons
            .apply(PlayerCommand::PurgeTube(0), &state, &tracks, config)
            .unwrap();
        weapons.update(config.purge_seconds, &mut world, config, &state);
        assert_eq!(weapons.tubes[0].phase, TubePhase::Empty);
        assert_eq!(weapons.reserve, 4);
    }

    #[test]
    fn simultaneous_torpedoes_keep_distinct_runs_and_both_expire() {
        let (mut weapons, mut world, state, config, tracks) = ready();
        world.vessels.clear();
        let settings = weapons.tubes[0].settings.unwrap();
        for (command, wait) in [
            (PlayerCommand::LoadTube(1), config.load_seconds),
            (PlayerCommand::ConfigureShot { tube: 1, settings }, 0.0),
            (PlayerCommand::FloodTube(1), config.flood_seconds),
            (PlayerCommand::OpenTube(1), config.door_seconds),
        ] {
            weapons.apply(command, &state, &tracks, config).unwrap();
            weapons.update(wait, &mut world, config, &state);
        }
        for tube in 0..2 {
            weapons
                .apply(
                    PlayerCommand::AuthorizeShot { tube, settings },
                    &state,
                    &tracks,
                    config,
                )
                .unwrap();
            weapons
                .apply(PlayerCommand::FireTorpedo { tube }, &state, &tracks, config)
                .unwrap();
        }
        let runs = weapons.measurements(&state, &tracks, config).torpedoes;
        assert_eq!(runs.len(), 2);
        assert_ne!(runs[0].id, runs[1].id);
        let events = weapons.update(400.0, &mut world, config, &state);
        assert_eq!(
            events,
            vec![
                GameEvent::TorpedoExpired {
                    torpedo_id: runs[0].id
                },
                GameEvent::TorpedoExpired {
                    torpedo_id: runs[1].id
                }
            ]
        );
        assert!(weapons.torpedoes.is_empty());
        assert_eq!(weapons.reserve, 3);
    }

    #[test]
    fn equal_and_nearly_equal_speeds_solve_approaching_targets_without_unstable_roots() {
        let state = SubmarineState::default();
        let config = WeaponsConfig::default();
        let settings = ShotSettings::for_target(TrackId(42));
        for speed in [
            config.torpedo_speed_knots,
            config.torpedo_speed_knots - 0.00001,
            config.torpedo_speed_knots + 0.00001,
        ] {
            let approaching = TrackEstimate {
                heading: Some(180.0),
                speed: Some(speed),
                ..track()
            };
            let solution = solution(&state, &[approaching], settings, config).unwrap();
            let expected =
                1000.0 / ((speed + config.torpedo_speed_knots) * KNOTS_TO_METERS_PER_SECOND);
            assert!((solution.run_seconds - expected).abs() < 0.001);
            assert!(crate::shortest_angle(0.0, solution.bearing).abs() < 0.001);
        }
        let receding = TrackEstimate {
            heading: Some(0.0),
            speed: Some(config.torpedo_speed_knots),
            ..track()
        };
        assert_eq!(
            solution(&state, &[receding], settings, config),
            Err(ProtocolError::NoFiringSolution)
        );
    }
}
