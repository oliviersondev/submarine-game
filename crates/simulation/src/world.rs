use shared::{AcousticProfile, SubmarineState};

use crate::KNOTS_TO_METERS_PER_SECOND;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VesselKind {
    Cargo,
    Escort,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Vessel {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) heading: f32,
    pub(crate) speed: f32,
    pub(crate) signature: f32,
    integrity: f32,
    kind: VesselKind,
}

impl Vessel {
    pub(crate) fn afloat(&self) -> bool {
        self.integrity > 0.0
    }
    pub(crate) fn profile(&self) -> AcousticProfile {
        match self.kind {
            VesselKind::Cargo => AcousticProfile::LowFrequency,
            VesselKind::Escort => AcousticProfile::HighFrequency,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct EnemyTrack {
    x: f32,
    y: f32,
    uncertainty: f32,
    observed_tick: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct World {
    seed: u64,
    pub(crate) vessels: Vec<Vessel>,
    escort_track: Option<EnemyTrack>,
}

impl World {
    pub(crate) fn new(seed: u64) -> Self {
        let offset = signed_unit(seed, 1) * 400.0;
        let heading = 165.0 + signed_unit(seed, 2) * 8.0;
        Self {
            seed,
            vessels: vec![
                vessel(VesselKind::Cargo, -550.0 + offset, 4_600.0, heading, 8.0),
                vessel(VesselKind::Cargo, 550.0 + offset, 4_850.0, heading, 7.5),
                vessel(VesselKind::Escort, offset, 4_050.0, heading, 11.0),
            ],
            escort_track: None,
        }
    }

    pub(crate) fn update(&mut self, dt: f32) {
        for vessel in &mut self.vessels {
            if vessel.integrity <= 0.0 {
                continue;
            }
            let distance = vessel.speed * KNOTS_TO_METERS_PER_SECOND * dt;
            let heading = vessel.heading.to_radians();
            vessel.x += heading.sin() * distance;
            vessel.y += heading.cos() * distance;

            if vessel.x.abs() > 7_000.0 || vessel.y.abs() > 7_000.0 {
                vessel.heading = (vessel.heading + 180.0).rem_euclid(360.0);
            }
        }
    }

    pub(crate) fn observe_active_ping(&mut self, submarine: &SubmarineState, tick: u64) {
        let noise_x = signed_unit(self.seed ^ tick, 31) * 240.0;
        let noise_y = signed_unit(self.seed ^ tick, 32) * 240.0;
        self.escort_track = Some(EnemyTrack {
            x: submarine.x + noise_x,
            y: submarine.y + noise_y,
            uncertainty: 350.0,
            observed_tick: tick,
        });
    }

    pub(crate) fn impact(
        &mut self,
        ax: f32,
        ay: f32,
        bx: f32,
        by: f32,
        depth: f32,
        config: shared::WeaponsConfig,
    ) -> Option<bool> {
        if depth > config.max_surface_impact_depth {
            return None;
        }
        let dx = bx - ax;
        let dy = by - ay;
        let length = dx * dx + dy * dy;
        let hit = self.vessels.iter().position(|v| {
            if v.integrity <= 0.0 {
                return false;
            }
            let t = if length > 0.0 {
                (((v.x - ax) * dx + (v.y - ay) * dy) / length).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (v.x - ax - t * dx).hypot(v.y - ay - t * dy) <= config.collision_radius_meters
        })?;
        self.vessels[hit].integrity = (self.vessels[hit].integrity - config.damage).max(0.0);
        let sunk = self.vessels[hit].integrity == 0.0;
        if sunk {
            self.vessels[hit].signature = 0.0;
            self.vessels[hit].speed = 0.0;
        }
        // Les bâtiments voisins observent l'explosion, sans connaître le tireur.
        let (x, y) = (self.vessels[hit].x, self.vessels[hit].y);
        for vessel in &mut self.vessels {
            if vessel.integrity > 0.0 && (vessel.x - x).hypot(vessel.y - y) < 3000.0 {
                vessel.heading = (vessel.heading + 35.0).rem_euclid(360.0);
            }
        }
        Some(sunk)
    }

    #[cfg(test)]
    pub(crate) fn enemy_track(&self) -> Option<(f32, f32, f32, u64)> {
        self.escort_track
            .as_ref()
            .map(|track| (track.x, track.y, track.uncertainty, track.observed_tick))
    }
}

fn vessel(kind: VesselKind, x: f32, y: f32, heading: f32, speed: f32) -> Vessel {
    Vessel {
        x,
        y,
        heading,
        speed,
        signature: match kind {
            VesselKind::Cargo => 82.0,
            VesselKind::Escort => 72.0,
        },
        integrity: 100.0,
        kind,
    }
}

pub(crate) fn signed_unit(seed: u64, channel: u64) -> f32 {
    let mut value = seed ^ channel.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value as u32 as f32 / u32::MAX as f32) * 2.0 - 1.0
}
