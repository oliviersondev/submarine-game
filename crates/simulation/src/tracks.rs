use shared::{
    AcousticProfile, ContactClassification, ObservationMode, SubmarineState, TrackEstimate, TrackId,
};

use crate::{detection::DetectionSample, KNOTS_TO_METERS_PER_SECOND};

const TRACK_LIFETIME_TICKS: u64 = 1_200;
const MAX_TRACK_SPEED_KNOTS: f32 = 40.0;

#[derive(Clone, Debug)]
struct Track {
    estimate: TrackEstimate,
    x: f32,
    y: f32,
    range_uncertainty: f32,
    observation_count: u32,
    previous_measurement: Option<(f32, f32, f32)>,
    last_observation_mode: ObservationMode,
    merchant_evidence: u8,
    escort_evidence: u8,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Tracker {
    tracks: Vec<Track>,
    next_id: u64,
    elapsed_seconds: f32,
}

impl Tracker {
    pub(crate) fn update(&mut self, tick: u64, submarine: &SubmarineState, dt: f32) {
        self.elapsed_seconds += dt;
        for track in &mut self.tracks {
            let age = tick.saturating_sub(track.estimate.last_observation_tick);
            if age > 0 {
                if let (Some(heading), Some(speed)) = (track.estimate.heading, track.estimate.speed)
                {
                    let distance = speed * KNOTS_TO_METERS_PER_SECOND * dt;
                    let heading = heading.to_radians();
                    track.x += heading.sin() * distance;
                    track.y += heading.cos() * distance;
                }
                track.estimate.confidence = (track.estimate.confidence - 0.012).max(0.0);
                track.estimate.bearing_uncertainty =
                    (track.estimate.bearing_uncertainty + 0.004).min(45.0);
                track.range_uncertainty = (track.range_uncertainty + dt * 10.0).min(10_000.0);
                if let Some(uncertainty) = &mut track.estimate.distance_uncertainty {
                    *uncertainty = track.range_uncertainty;
                }
                refresh_geometry(track, submarine);
            }
        }
        self.tracks.retain(|track| {
            tick.saturating_sub(track.estimate.last_observation_tick) <= TRACK_LIFETIME_TICKS
                && track.estimate.confidence > 0.0
        });
    }

    pub(crate) fn associate(&mut self, sample: &DetectionSample, submarine: &SubmarineState) {
        let observation = &sample.observation;
        let candidate = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| {
                !(track.estimate.last_observation_tick == observation.tick
                    && track.last_observation_mode == observation.mode)
                    && observation
                        .tick
                        .saturating_sub(track.estimate.last_observation_tick)
                        <= 120
            })
            .map(|(index, track)| {
                let predicted_range = (track.x - submarine.x).hypot(track.y - submarine.y);
                let angle = angular_distance(track.estimate.bearing, observation.bearing);
                let range = (predicted_range - sample.range_hint).abs();
                (index, angle, range, angle + range / 400.0)
            })
            .filter(|(_, angle, range, _)| {
                *angle <= 9.0 && *range <= (sample.range_uncertainty * 1.5).max(550.0)
            })
            .min_by(|left, right| left.3.total_cmp(&right.3).then(left.0.cmp(&right.0)))
            .map(|(index, _, _, _)| index);

        if let Some(index) = candidate {
            update_track(
                &mut self.tracks[index],
                sample,
                submarine,
                self.elapsed_seconds,
            );
        } else {
            let id = TrackId(self.next_id);
            self.next_id = self.next_id.wrapping_add(1);
            let mut track = Track {
                estimate: TrackEstimate {
                    id,
                    bearing: observation.bearing,
                    bearing_uncertainty: observation.bearing_uncertainty,
                    distance: observation.distance,
                    distance_uncertainty: observation.distance_uncertainty,
                    heading: None,
                    speed: None,
                    classification: ContactClassification::Unknown,
                    confidence: if observation.mode == ObservationMode::Active {
                        48.0
                    } else {
                        24.0
                    },
                    last_observation_tick: observation.tick,
                    shared: false,
                },
                x: 0.0,
                y: 0.0,
                range_uncertainty: sample.range_uncertainty,
                observation_count: 1,
                previous_measurement: None,
                last_observation_mode: observation.mode,
                merchant_evidence: 0,
                escort_evidence: 0,
            };
            let (x, y) = measured_position(sample, submarine);
            track.x = x;
            track.y = y;
            track.previous_measurement = Some((x, y, self.elapsed_seconds));
            if observation.mode == ObservationMode::Active {
                track.estimate.distance = Some(sample.range_hint);
                track.estimate.distance_uncertainty = Some(sample.range_uncertainty);
            }
            add_evidence(&mut track, observation.profile);
            self.tracks.push(track);
        }
    }

    pub(crate) fn estimates(&self) -> Vec<TrackEstimate> {
        self.tracks
            .iter()
            .map(|track| track.estimate.clone())
            .collect()
    }

    pub(crate) fn shared_estimates(&self) -> Vec<TrackEstimate> {
        self.tracks
            .iter()
            .filter(|track| track.estimate.shared)
            .map(|track| track.estimate.clone())
            .collect()
    }

    pub(crate) fn set_shared(&mut self, id: TrackId, shared: bool) -> Result<(), TrackError> {
        let track = self.find_mut(id)?;
        track.estimate.shared = shared;
        Ok(())
    }

    pub(crate) fn drop_track(&mut self, id: TrackId) -> Result<(), TrackError> {
        let Some(index) = self.tracks.iter().position(|track| track.estimate.id == id) else {
            return Err(TrackError::NotFound(id));
        };
        self.tracks.remove(index);
        Ok(())
    }

    pub(crate) fn merge(&mut self, primary: TrackId, secondary: TrackId) -> Result<(), TrackError> {
        if primary == secondary {
            return Err(TrackError::InvalidMerge);
        }
        let Some(primary_index) = self
            .tracks
            .iter()
            .position(|track| track.estimate.id == primary)
        else {
            return Err(TrackError::NotFound(primary));
        };
        let Some(secondary_index) = self
            .tracks
            .iter()
            .position(|track| track.estimate.id == secondary)
        else {
            return Err(TrackError::NotFound(secondary));
        };
        if angular_distance(
            self.tracks[primary_index].estimate.bearing,
            self.tracks[secondary_index].estimate.bearing,
        ) > 35.0
        {
            return Err(TrackError::InvalidMerge);
        }

        let secondary_track = self.tracks.remove(secondary_index);
        let adjusted_primary = if secondary_index < primary_index {
            primary_index - 1
        } else {
            primary_index
        };
        let primary_track = &mut self.tracks[adjusted_primary];
        primary_track.estimate.bearing = mean_bearing(
            primary_track.estimate.bearing,
            secondary_track.estimate.bearing,
        );
        primary_track.estimate.bearing_uncertainty = primary_track
            .estimate
            .bearing_uncertainty
            .min(secondary_track.estimate.bearing_uncertainty);
        if primary_track.estimate.distance.is_none() {
            primary_track.estimate.distance = secondary_track.estimate.distance;
            primary_track.estimate.distance_uncertainty =
                secondary_track.estimate.distance_uncertainty;
        }
        primary_track.estimate.confidence = (primary_track.estimate.confidence
            + secondary_track.estimate.confidence * 0.5)
            .min(100.0);
        primary_track.estimate.shared |= secondary_track.estimate.shared;
        primary_track.x = (primary_track.x + secondary_track.x) * 0.5;
        primary_track.y = (primary_track.y + secondary_track.y) * 0.5;
        primary_track.range_uncertainty = primary_track
            .range_uncertainty
            .min(secondary_track.range_uncertainty);
        primary_track.observation_count = primary_track
            .observation_count
            .saturating_add(secondary_track.observation_count);
        if secondary_track.estimate.last_observation_tick
            > primary_track.estimate.last_observation_tick
        {
            primary_track.estimate.last_observation_tick =
                secondary_track.estimate.last_observation_tick;
            primary_track.previous_measurement = secondary_track.previous_measurement;
            primary_track.last_observation_mode = secondary_track.last_observation_mode;
            primary_track.estimate.heading = secondary_track.estimate.heading;
            primary_track.estimate.speed = secondary_track.estimate.speed;
        }
        primary_track.merchant_evidence = primary_track
            .merchant_evidence
            .saturating_add(secondary_track.merchant_evidence);
        primary_track.escort_evidence = primary_track
            .escort_evidence
            .saturating_add(secondary_track.escort_evidence);
        classify(primary_track);
        Ok(())
    }

    fn find_mut(&mut self, id: TrackId) -> Result<&mut Track, TrackError> {
        self.tracks
            .iter_mut()
            .find(|track| track.estimate.id == id)
            .ok_or(TrackError::NotFound(id))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrackError {
    NotFound(TrackId),
    InvalidMerge,
}

fn update_track(
    track: &mut Track,
    sample: &DetectionSample,
    submarine: &SubmarineState,
    elapsed_seconds: f32,
) {
    let observation = &sample.observation;
    let (measured_x, measured_y) = measured_position(sample, submarine);
    if let Some((previous_x, previous_y, previous_time)) = track.previous_measurement {
        let elapsed = elapsed_seconds - previous_time;
        if elapsed > 0.0 {
            let dx = measured_x - previous_x;
            let dy = measured_y - previous_y;
            let measured_speed = (dx.hypot(dy) / elapsed / KNOTS_TO_METERS_PER_SECOND)
                .clamp(0.0, MAX_TRACK_SPEED_KNOTS);
            let measured_heading = dx.atan2(dy).to_degrees().rem_euclid(360.0);
            track.estimate.heading = Some(match track.estimate.heading {
                Some(previous) => mean_bearing(previous, measured_heading),
                None => measured_heading,
            });
            track.estimate.speed = Some(match track.estimate.speed {
                Some(previous) => previous * 0.65 + measured_speed * 0.35,
                None => measured_speed,
            });
        }
    }
    track.previous_measurement = Some((measured_x, measured_y, elapsed_seconds));
    track.last_observation_mode = observation.mode;
    track.x = track.x * 0.45 + measured_x * 0.55;
    track.y = track.y * 0.45 + measured_y * 0.55;
    track.observation_count = track.observation_count.saturating_add(1);
    track.range_uncertainty = if observation.mode == ObservationMode::Active {
        sample.range_uncertainty
    } else {
        (sample.range_uncertainty / (track.observation_count as f32).sqrt()).max(120.0)
    };
    track.estimate.bearing = mean_bearing(track.estimate.bearing, observation.bearing);
    track.estimate.bearing_uncertainty =
        (track.estimate.bearing_uncertainty * 0.65).min(observation.bearing_uncertainty);
    if observation.mode == ObservationMode::Active || track.observation_count >= 3 {
        let distance = (track.x - submarine.x).hypot(track.y - submarine.y);
        track.estimate.distance = Some(distance);
        track.estimate.distance_uncertainty = Some(track.range_uncertainty);
    }
    track.estimate.confidence = (track.estimate.confidence
        + if observation.mode == ObservationMode::Active {
            24.0
        } else {
            12.0
        })
    .min(100.0);
    track.estimate.last_observation_tick = observation.tick;
    refresh_geometry(track, submarine);
    add_evidence(track, observation.profile);
}

fn measured_position(sample: &DetectionSample, submarine: &SubmarineState) -> (f32, f32) {
    let bearing = sample.observation.bearing.to_radians();
    (
        submarine.x + bearing.sin() * sample.range_hint,
        submarine.y + bearing.cos() * sample.range_hint,
    )
}

fn refresh_geometry(track: &mut Track, submarine: &SubmarineState) {
    let dx = track.x - submarine.x;
    let dy = track.y - submarine.y;
    track.estimate.bearing = dx.atan2(dy).to_degrees().rem_euclid(360.0);
    if track.estimate.distance.is_some() {
        track.estimate.distance = Some(dx.hypot(dy));
    }
}

fn add_evidence(track: &mut Track, profile: AcousticProfile) {
    match profile {
        AcousticProfile::LowFrequency => {
            track.merchant_evidence = track.merchant_evidence.saturating_add(1)
        }
        AcousticProfile::HighFrequency => {
            track.escort_evidence = track.escort_evidence.saturating_add(1)
        }
        AcousticProfile::Mixed => {
            track.merchant_evidence = track.merchant_evidence.saturating_add(1);
            track.escort_evidence = track.escort_evidence.saturating_add(1);
        }
    }
    classify(track);
}

fn classify(track: &mut Track) {
    track.estimate.classification =
        if track.merchant_evidence >= 3 && track.merchant_evidence > track.escort_evidence {
            ContactClassification::Merchant
        } else if track.escort_evidence >= 3 && track.escort_evidence > track.merchant_evidence {
            ContactClassification::Escort
        } else {
            ContactClassification::Unknown
        };
}

fn angular_distance(first: f32, second: f32) -> f32 {
    ((first - second + 180.0).rem_euclid(360.0) - 180.0).abs()
}

fn mean_bearing(first: f32, second: f32) -> f32 {
    let delta = (second - first + 180.0).rem_euclid(360.0) - 180.0;
    (first + delta * 0.5).rem_euclid(360.0)
}
