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
}

impl Weapons {
    pub(crate) fn new() -> Self {
        Self {
            tubes: (0..5)
                .map(|_| TubeMeasurements {
                    phase: TubePhase::Empty,
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
        WeaponsMeasurements {
            tubes: self.tubes.clone(),
            reserve: self.reserve,
            solutions: self
                .tubes
                .iter()
                .enumerate()
                .map(|(index, tube)| {
                    tube.settings
                        .and_then(|settings| solution(state, tracks, settings, config).ok())
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
            let index = self
                .tubes
                .iter()
                .position(|t| t.phase == Loaded || t.phase == Empty)
                .ok_or(ProtocolError::InvalidTubePhase)?;
            let settings = ShotSettings::for_target(target);
            if self.tubes[index].phase == Empty {
                self.apply(LoadTube(index as u8), state, tracks, config)?;
            }
            self.tubes[index].settings = Some(settings);
            self.tubes[index].authorized = false;
            self.bot_tube = Some(index as u8);
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
            ConfigureShot { settings, .. } if matches!(tube.phase, Loaded | Flooded | Ready) => {
                validate_settings(settings)?;
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
            AuthorizeShot { settings, .. } if tube.settings == Some(settings) => {
                validate_settings(settings)?;
                solution(state, tracks, settings, config)?;
                tube.authorized = true;
            }
            FireTorpedo { .. } if tube.phase == Ready => {
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
                    return Err(ProtocolError::NoFiringSolution);
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
        let command = match tube.phase {
            TubePhase::Loaded => Some(PlayerCommand::FloodTube(index)),
            TubePhase::Flooded if tube.settings.is_some() => Some(PlayerCommand::OpenTube(index)),
            TubePhase::Flooded => Some(PlayerCommand::PurgeTube(index)),
            TubePhase::Ready if tube.authorized => Some(PlayerCommand::FireTorpedo { tube: index }),
            TubePhase::Empty => {
                self.bot_tube = None;
                None
            }
            _ => None,
        };
        command
            .and_then(|c| self.apply(c, state, tracks, config).ok())
            .unwrap_or_default()
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
                    TubePhase::Loading => TubePhase::Loaded,
                    TubePhase::Flooding => TubePhase::Flooded,
                    TubePhase::Opening => TubePhase::Ready,
                    TubePhase::Closing => TubePhase::Flooded,
                    TubePhase::Purging if tube.settings.is_some() => TubePhase::Loaded,
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
        self.bot_tube.is_none() && self.reserve > 0
    }
}

fn validate_settings(settings: ShotSettings) -> Result<(), ProtocolError> {
    if !settings.depth.is_finite()
        || !(1.0..=15.0).contains(&settings.depth)
        || !settings.heading_correction.is_finite()
        || settings.heading_correction.abs() > 45.0
        || !settings.speed_correction.is_finite()
        || settings.speed_correction.abs() > 10.0
    {
        return Err(ProtocolError::InvalidShotSettings);
    }
    Ok(())
}

pub(crate) fn solution(
    state: &SubmarineState,
    tracks: &[TrackEstimate],
    settings: ShotSettings,
    config: WeaponsConfig,
) -> Result<FiringSolution, ProtocolError> {
    validate_settings(settings)?;
    let track = tracks
        .iter()
        .find(|t| t.id == settings.target)
        .ok_or(ProtocolError::TrackNotShared)?;
    if !track.shared {
        return Err(ProtocolError::TrackNotShared);
    }
    let distance = track.distance.ok_or(ProtocolError::NoFiringSolution)?;
    let heading = (track.heading.ok_or(ProtocolError::NoFiringSolution)?
        + settings.heading_correction)
        .to_radians();
    let speed = (track.speed.ok_or(ProtocolError::NoFiringSolution)? + settings.speed_correction)
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
    if discriminant < 0.0 || a.abs() < 0.0001 {
        return Err(ProtocolError::NoFiringSolution);
    }
    let roots = [
        (-b + discriminant.sqrt()) / (2.0 * a),
        (-b - discriminant.sqrt()) / (2.0 * a),
    ];
    let time = roots
        .into_iter()
        .filter(|t| t.is_finite() && *t > 0.0)
        .reduce(f32::min)
        .ok_or(ProtocolError::NoFiringSolution)?;
    if time * ts > config.torpedo_range_meters {
        return Err(ProtocolError::NoFiringSolution);
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
    if settings.depth > 8.0 {
        warnings.push("Torpille sous la coque des navires de surface".into());
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
            world.update(0.05);
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
        for (changed, error) in [(true, 0.0), (false, 15.0)] {
            let (events, world) = attack(changed, error);
            assert!(world.vessels[0].afloat());
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
}
