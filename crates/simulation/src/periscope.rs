use shared::{
    PeriscopeBlock, PeriscopeConfig, PeriscopeMeasurements, PlayerCommand, ProtocolError,
    SilhouetteKind, SubmarineState, VisualContact, VisualContactId, VisualSilhouette,
};

use crate::world::World;

const MAX_CONTACTS: usize = 64;

#[derive(Default)]
pub(crate) struct Periscope {
    raised: bool,
    bearing: f32,
    elapsed: f64,
    blocked: Option<PeriscopeBlock>,
    last_auto_lowering: Option<PeriscopeBlock>,
    visible: Vec<(usize, VisualSilhouette)>,
    contacts: Vec<(Option<usize>, VisualContact)>,
    next_observation: u64,
    next_contact: u64,
    origin: (f32, f32),
    contact_origins: std::collections::HashMap<VisualContactId, (f32, f32)>,
}

impl Periscope {
    pub(crate) fn raised(&self) -> bool {
        self.raised
    }

    pub(crate) fn observe_explosion(
        &self,
        x: f32,
        y: f32,
        submarine: &SubmarineState,
        config: PeriscopeConfig,
    ) -> Option<shared::GameEvent> {
        if !self.raised || self.blocked.is_some() {
            return None;
        }
        let config = valid_config(config);
        let dx = x - submarine.x;
        let dy = y - submarine.y;
        let distance = dx.hypot(dy);
        let bearing = dx.atan2(dy).to_degrees().rem_euclid(360.0);
        if !distance.is_finite()
            || distance > config.range
            || angle_delta(bearing, self.bearing).abs() > config.field_of_view * 0.5
        {
            return None;
        }
        Some(shared::GameEvent::SurfaceExplosionObserved {
            bearing,
            // Stylised 30m surface plume; angular measure only, no world coordinates.
            angular_size: (2.0 * 15.0_f32.atan2(distance.max(0.1))).to_degrees(),
        })
    }

    pub(crate) fn update(
        &mut self,
        world: &World,
        submarine: &SubmarineState,
        config: PeriscopeConfig,
        destroyed: bool,
        dt: f32,
    ) {
        let config = valid_config(config);
        self.elapsed += f64::from(dt);
        self.origin = (submarine.x, submarine.y);
        self.blocked = if destroyed {
            Some(PeriscopeBlock::Destroyed)
        } else if !submarine.depth.is_finite()
            || submarine.depth < 0.0
            || submarine.depth > config.depth + config.depth_tolerance
        {
            Some(PeriscopeBlock::Depth)
        } else if !submarine.speed.is_finite() || submarine.speed.abs() > config.max_speed {
            Some(PeriscopeBlock::Speed)
        } else {
            None
        };
        if self.raised && self.blocked.is_some() {
            self.raised = false;
            self.last_auto_lowering = self.blocked;
        }
        let mut visible = Vec::new();
        if self.raised {
            for (index, vessel) in world.vessels.iter().enumerate() {
                if !vessel.afloat() {
                    continue;
                }
                let dx = vessel.x - submarine.x;
                let dy = vessel.y - submarine.y;
                let distance = dx.hypot(dy);
                let bearing = dx.atan2(dy).to_degrees().rem_euclid(360.0);
                if !distance.is_finite()
                    || distance > config.range
                    || angle_delta(bearing, self.bearing).abs() > config.field_of_view * 0.5
                {
                    continue;
                }
                let kind = vessel.silhouette(index);
                let aspect = (vessel.heading - bearing).to_radians().sin();
                let length = match kind {
                    SilhouetteKind::Escort => 95.0,
                    _ => 140.0,
                };
                let width = 14.0 + (length - 14.0) * aspect.abs();
                let observation_id = self
                    .visible
                    .iter()
                    .find(|(i, _)| *i == index)
                    .map(|(_, s)| s.observation_id)
                    .unwrap_or_else(|| {
                        self.next_observation += 1;
                        self.next_observation
                    });
                visible.push((
                    index,
                    VisualSilhouette {
                        observation_id,
                        contact_id: None,
                        bearing,
                        aspect,
                        kind,
                        angular_height: (2.0 * (12.0_f32).atan2(distance.max(0.1))).to_degrees(),
                        angular_width: (2.0 * (width * 0.5).atan2(distance.max(0.1))).to_degrees(),
                    },
                ));
            }
        }
        self.visible = visible;
        for (vessel, contact) in &mut self.contacts {
            if let Some((_, silhouette)) = self.visible.iter().find(|(i, _)| Some(*i) == *vessel) {
                contact.bearing = silhouette.bearing;
                contact.kind = silhouette.kind;
                contact.observed_at = self.elapsed;
                self.contact_origins.insert(contact.id, self.origin);
            } else {
                // Never reconnect a lost contact using its private vessel identity.
                *vessel = None;
            }
            contact.age = (self.elapsed - contact.observed_at) as f32;
            contact.old = contact.age >= config.old_after;
        }
        self.contacts.retain(|(_, c)| c.age < config.expires_after);
        self.contact_origins
            .retain(|id, _| self.contacts.iter().any(|(_, c)| c.id == *id));
    }

    pub(crate) fn command(&mut self, command: PlayerCommand) -> Result<(), ProtocolError> {
        match command {
            PlayerCommand::SetPeriscopeRaised(raised) => {
                if raised {
                    if let Some(reason) = self.blocked {
                        return Err(ProtocolError::PeriscopeUnavailable(reason));
                    }
                }
                self.raised = raised;
                self.last_auto_lowering = None;
            }
            PlayerCommand::SetPeriscopeBearing(bearing) => {
                if !bearing.is_finite() {
                    return Err(ProtocolError::InvalidPeriscopeBearing);
                }
                self.bearing = bearing.rem_euclid(360.0);
            }
            PlayerCommand::CreateVisualContact { observation_id } => {
                let (index, silhouette) = self
                    .visible
                    .iter()
                    .find(|(_, s)| s.observation_id == observation_id)
                    .ok_or(ProtocolError::VisualObservationLost)?;
                if self.contacts.iter().any(|(i, _)| *i == Some(*index)) {
                    return Ok(());
                }
                if self.contacts.len() >= MAX_CONTACTS {
                    return Err(ProtocolError::VisualContactLimitReached);
                }
                self.next_contact += 1;
                self.contact_origins
                    .insert(VisualContactId(self.next_contact), self.origin);
                self.contacts.push((
                    Some(*index),
                    VisualContact {
                        id: VisualContactId(self.next_contact),
                        bearing: silhouette.bearing,
                        kind: silhouette.kind,
                        observed_at: self.elapsed,
                        age: 0.0,
                        old: false,
                        shared: false,
                    },
                ));
            }
            PlayerCommand::ShareVisualContact(id) => {
                let (_, contact) = self
                    .contacts
                    .iter_mut()
                    .find(|(_, c)| c.id == id)
                    .ok_or(ProtocolError::VisualContactNotFound)?;
                contact.shared = true;
            }
            _ => unreachable!("periscope commands only"),
        }
        Ok(())
    }

    pub(crate) fn measurements(&self, config: PeriscopeConfig) -> PeriscopeMeasurements {
        PeriscopeMeasurements {
            raised: self.raised,
            bearing: self.bearing,
            field_of_view: valid_config(config).field_of_view,
            blocked: self.blocked,
            last_auto_lowering: self.last_auto_lowering,
            silhouettes: self
                .visible
                .iter()
                .map(|(index, s)| {
                    let mut silhouette = s.clone();
                    silhouette.contact_id = self
                        .contacts
                        .iter()
                        .find(|(i, _)| *i == Some(*index))
                        .map(|(_, c)| c.id);
                    silhouette
                })
                .collect(),
        }
    }

    pub(crate) fn contacts(&self, shared_only: bool) -> Vec<VisualContact> {
        self.contacts
            .iter()
            .filter(|(_, c)| !shared_only || c.shared)
            .map(|(_, c)| c.clone())
            .collect()
    }

    pub(crate) fn firing_observation(
        &self,
        id: VisualContactId,
    ) -> Result<(VisualContact, (f32, f32)), ProtocolError> {
        let contact = self
            .contacts
            .iter()
            .find(|(_, c)| c.id == id)
            .ok_or(ProtocolError::VisualContactNotFound)?;
        let origin = self
            .contact_origins
            .get(&id)
            .ok_or(ProtocolError::VisualContactNotFound)?;
        Ok((contact.1.clone(), *origin))
    }
}

fn valid_config(config: PeriscopeConfig) -> PeriscopeConfig {
    if [
        config.depth,
        config.depth_tolerance,
        config.max_speed,
        config.field_of_view,
        config.range,
        config.old_after,
        config.expires_after,
    ]
    .iter()
    .all(|v| v.is_finite())
        && config.depth >= 0.0
        && config.depth_tolerance >= 0.0
        && config.max_speed >= 0.0
        && config.field_of_view > 0.0
        && config.field_of_view <= 360.0
        && config.range > 0.0
        && config.old_after >= 0.0
        && config.expires_after > config.old_after
    {
        config
    } else {
        PeriscopeConfig::default()
    }
}

pub(crate) fn angle_delta(bearing: f32, origin: f32) -> f32 {
    (bearing - origin + 180.0).rem_euclid(360.0) - 180.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explosion_observation_respects_range_field_wrap_and_automatic_lowering() {
        let (mut p, w, mut s, c) = scene();
        raise(&mut p, &w, &s, c);
        assert!(p.observe_explosion(0.0, c.range, &s, c).is_some());
        assert!(p.observe_explosion(0.0, c.range + 1.0, &s, c).is_none());
        assert!(p.observe_explosion(1000.0, 0.0, &s, c).is_none());
        p.command(PlayerCommand::SetPeriscopeBearing(359.0))
            .unwrap();
        assert!(p.observe_explosion(0.0, 1000.0, &s, c).is_some());
        s.speed = c.max_speed + 1.0;
        p.update(&w, &s, c, false, 0.0);
        assert!(p.observe_explosion(0.0, 1000.0, &s, c).is_none());
        s.speed = 0.0;
        raise(&mut p, &w, &s, c);
        p.update(&w, &s, c, true, 0.0);
        assert!(p.observe_explosion(0.0, 1000.0, &s, c).is_none());
    }
    fn scene() -> (Periscope, World, SubmarineState, PeriscopeConfig) {
        let mut state = SubmarineState::default();
        state.depth = 12.0;
        let mut world = World::new(1);
        world.vessels.truncate(1);
        world.vessels[0].x = 0.0;
        world.vessels[0].y = 4_000.0;
        (
            Periscope::default(),
            world,
            state,
            PeriscopeConfig::default(),
        )
    }
    fn raise(p: &mut Periscope, w: &World, s: &SubmarineState, c: PeriscopeConfig) {
        p.update(w, s, c, false, 0.0);
        p.command(PlayerCommand::SetPeriscopeRaised(true)).unwrap();
        p.update(w, s, c, false, 0.0);
    }
    #[test]
    fn limits_are_inclusive_and_auto_lowering_does_not_reverse() {
        let (mut p, w, mut s, c) = scene();
        for depth in [0.0, 0.5, 5.0, 10.0, 12.0, 14.0] {
            s.depth = depth;
            s.speed = 6.0;
            raise(&mut p, &w, &s, c);
            assert!(p.raised);
            assert!(!p.visible.is_empty());
        }
        for depth in [12.0, 5.0, 0.0] {
            s.depth = depth;
            p.update(&w, &s, c, false, 0.05);
            assert!(p.raised && !p.visible.is_empty());
        }
        s.speed = 6.01;
        p.update(&w, &s, c, false, 0.05);
        assert!(!p.raised && p.visible.is_empty());
        assert_eq!(p.last_auto_lowering, Some(PeriscopeBlock::Speed));
        s.speed = 0.0;
        p.update(&w, &s, c, false, 0.05);
        assert!(!p.raised);
        s.depth = 14.01;
        p.update(&w, &s, c, false, 0.0);
        assert_eq!(
            p.command(PlayerCommand::SetPeriscopeRaised(true)),
            Err(ProtocolError::PeriscopeUnavailable(PeriscopeBlock::Depth))
        );
        s.depth = 12.0;
        raise(&mut p, &w, &s, c);
        s.depth = 14.01;
        p.update(&w, &s, c, false, 0.05);
        assert!(!p.raised && p.visible.is_empty());
        assert_eq!(p.last_auto_lowering, Some(PeriscopeBlock::Depth));
        s.depth = 0.0;
        p.update(&w, &s, c, false, 0.05);
        assert!(!p.raised);
        assert_eq!(p.blocked, None);
    }
    #[test]
    fn visibility_wrap_range_field_and_nonfinite_inputs() {
        let (mut p, mut w, s, c) = scene();
        p.command(PlayerCommand::SetPeriscopeBearing(359.0))
            .unwrap();
        raise(&mut p, &w, &s, c);
        assert_eq!(p.visible.len(), 1);
        p.command(PlayerCommand::SetPeriscopeBearing(21.0)).unwrap();
        p.update(&w, &s, c, false, 0.0);
        assert!(p.visible.is_empty());
        p.command(PlayerCommand::SetPeriscopeBearing(20.0)).unwrap();
        w.vessels[0].y = c.range;
        p.update(&w, &s, c, false, 0.0);
        assert_eq!(p.visible.len(), 1);
        w.vessels[0].y += 1.0;
        p.update(&w, &s, c, false, 0.0);
        assert!(p.visible.is_empty());
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                p.command(PlayerCommand::SetPeriscopeBearing(value)),
                Err(ProtocolError::InvalidPeriscopeBearing)
            );
        }
    }
    #[test]
    fn contacts_freeze_age_expire_and_never_reassociate_after_loss() {
        let (mut p, mut w, s, c) = scene();
        raise(&mut p, &w, &s, c);
        let token = p.visible[0].1.observation_id;
        p.command(PlayerCommand::CreateVisualContact {
            observation_id: token,
        })
        .unwrap();
        assert!(p.contacts(true).is_empty());
        p.command(PlayerCommand::ShareVisualContact(VisualContactId(1)))
            .unwrap();
        assert_eq!(p.contacts(true).len(), 1);
        p.command(PlayerCommand::SetPeriscopeRaised(false)).unwrap();
        p.update(&w, &s, c, false, 30.0);
        assert!(p.contacts(false)[0].old);
        assert_eq!(
            p.command(PlayerCommand::CreateVisualContact {
                observation_id: token
            }),
            Err(ProtocolError::VisualObservationLost)
        );
        w.vessels[0].x = 100.0;
        raise(&mut p, &w, &s, c);
        assert_ne!(p.visible[0].1.observation_id, token);
        assert_eq!(p.contacts(false)[0].bearing, 0.0);
        p.update(&w, &s, c, false, 90.0);
        assert!(p.contacts(false).is_empty());
    }
    #[test]
    fn apparent_size_orientation_and_destruction_are_observable_only() {
        let (mut p, mut w, s, c) = scene();
        w.vessels[0].heading = 90.0;
        raise(&mut p, &w, &s, c);
        let broadside = p.visible[0].1.angular_width;
        w.vessels[0].y *= 0.5;
        p.update(&w, &s, c, false, 0.0);
        assert!(p.visible[0].1.angular_width > broadside);
        w.vessels[0].heading = 0.0;
        p.update(&w, &s, c, false, 0.0);
        assert!(p.visible[0].1.angular_width < broadside);
        p.update(&w, &s, c, true, 0.0);
        assert!(p.visible.is_empty());
        assert_eq!(p.blocked, Some(PeriscopeBlock::Destroyed));
    }

    #[test]
    fn replay_is_deterministic_and_unseen_movement_cannot_change_contact() {
        fn run(hidden_x: f32) -> (PeriscopeMeasurements, Vec<VisualContact>) {
            let (mut p, mut w, s, c) = scene();
            raise(&mut p, &w, &s, c);
            let token = p.visible[0].1.observation_id;
            p.command(PlayerCommand::CreateVisualContact {
                observation_id: token,
            })
            .unwrap();
            p.command(PlayerCommand::SetPeriscopeRaised(false)).unwrap();
            p.update(&w, &s, c, false, 0.0);
            w.vessels[0].x = hidden_x;
            for _ in 0..100 {
                p.update(&w, &s, c, false, 0.05);
            }
            (p.measurements(c), p.contacts(false))
        }
        assert_eq!(run(500.0), run(500.0));
        assert_eq!(run(500.0), run(-500.0));
    }

    #[test]
    fn contacts_are_bounded_and_creation_is_idempotent_only_while_visible() {
        let (mut p, w, s, c) = scene();
        for _ in 0..MAX_CONTACTS {
            raise(&mut p, &w, &s, c);
            let token = p.visible[0].1.observation_id;
            let command = PlayerCommand::CreateVisualContact {
                observation_id: token,
            };
            p.command(command.clone()).unwrap();
            p.command(command).unwrap();
            let id = p.measurements(c).silhouettes[0].contact_id;
            assert!(id.is_some());
            p.command(PlayerCommand::SetPeriscopeRaised(false)).unwrap();
            p.update(&w, &s, c, false, 0.0);
        }
        assert_eq!(p.contacts(false).len(), MAX_CONTACTS);
        raise(&mut p, &w, &s, c);
        assert_eq!(
            p.command(PlayerCommand::CreateVisualContact {
                observation_id: p.visible[0].1.observation_id
            }),
            Err(ProtocolError::VisualContactLimitReached)
        );
        p.update(&w, &s, c, false, c.expires_after);
        assert!(p.contacts(false).is_empty());
        assert_eq!(
            p.command(PlayerCommand::ShareVisualContact(VisualContactId(1))),
            Err(ProtocolError::VisualContactNotFound)
        );
    }

    #[test]
    fn configuration_changes_limits_and_invalid_tuning_falls_back_safely() {
        let (mut p, w, mut s, mut c) = scene();
        c.depth = 20.0;
        c.max_speed = 3.0;
        s.depth = 22.01;
        p.update(&w, &s, c, false, 0.0);
        assert_eq!(p.blocked, Some(PeriscopeBlock::Depth));
        s.depth = 20.0;
        raise(&mut p, &w, &s, c);
        s.speed = 3.01;
        p.update(&w, &s, c, false, 0.0);
        assert_eq!(p.blocked, Some(PeriscopeBlock::Speed));
        c.field_of_view = f32::NAN;
        assert_eq!(valid_config(c), PeriscopeConfig::default());
    }

    #[test]
    fn invalid_depth_is_rejected_even_when_surface_observation_is_allowed() {
        let (mut p, w, mut s, c) = scene();
        for depth in [-0.01, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            s.depth = depth;
            p.update(&w, &s, c, false, 0.0);
            assert_eq!(
                p.command(PlayerCommand::SetPeriscopeRaised(true)),
                Err(ProtocolError::PeriscopeUnavailable(PeriscopeBlock::Depth))
            );
            assert!(p.visible.is_empty());
        }
    }
}
