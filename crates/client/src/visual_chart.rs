//! Visual chart geometry consumes observations and applied player hypotheses only.
use bevy::prelude::*;
use shared::{PeriscopeConfig, VisualContact, VisualTargetEstimate, WeaponsMeasurements};

use crate::chart::{direction, ChartView};

const KNOTS_TO_METERS_PER_SECOND: f32 = 0.514_444;

pub(crate) fn applied_position(weapons: &WeaponsMeasurements) -> Option<Vec2> {
    let target = weapons.visual_target?;
    let age = weapons.visual_target_age;
    if ![
        target.origin_x,
        target.origin_y,
        target.bearing,
        target.distance,
        target.heading,
        target.speed,
        age,
    ]
    .into_iter()
    .all(f32::is_finite)
        || target.distance <= 0.0
        || target.speed < 0.0
    {
        return None;
    }
    let position = Vec2::new(target.origin_x, target.origin_y)
        + direction(target.bearing) * target.distance
        + direction(target.heading) * target.speed * KNOTS_TO_METERS_PER_SECOND * age.max(0.0);
    position.is_finite().then_some(position)
}

pub(crate) fn observed_ray(view: &ChartView, contact: &VisualContact) -> Option<(Vec2, Vec2)> {
    let origin = Vec2::new(contact.origin_x, contact.origin_y);
    if !origin.is_finite() || !contact.bearing.is_finite() {
        return None;
    }
    // Reach the far edge even when the observation's own position is outside the viewport.
    let length = origin.distance(view.center) + view.size.length() / view.scale() * 2.0;
    view.clip_line(
        view.project(origin),
        view.project(origin + direction(contact.bearing) * length),
    )
}

pub(crate) fn estimate_is_old(age: f32, config: PeriscopeConfig) -> bool {
    let threshold = if config.old_after.is_finite() && config.old_after >= 0.0 {
        config.old_after
    } else {
        PeriscopeConfig::default().old_after
    };
    age >= threshold
}

pub(crate) fn estimate_label(
    target: VisualTargetEstimate,
    age: f32,
    old: bool,
    compact: bool,
) -> String {
    if compact {
        return format!(
            "V{} EST. r{} / {:.0}s\n{}",
            target.contact.0,
            target.revision,
            age.max(0.0),
            if old {
                "ANCIEN / HYP. MAN."
            } else {
                "HYP. MANUELLES"
            }
        );
    }
    format!(
        "V{} EST. r{} / {:.0}s{}\nVISUEL / HYPOTHESES MANUELLES",
        target.contact.0,
        target.revision,
        age.max(0.0),
        if old { " / ANCIEN" } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{SilhouetteKind, VisualContactId};

    fn weapons() -> WeaponsMeasurements {
        WeaponsMeasurements {
            visual_target: Some(VisualTargetEstimate {
                revision: 2,
                contact: VisualContactId(7),
                kind: SilhouetteKind::SingleFunnelCargo,
                bearing: 0.0,
                observed_at: 10.0,
                origin_x: 100.0,
                origin_y: -200.0,
                distance: 2_000.0,
                heading: 90.0,
                speed: 8.0,
            }),
            visual_target_age: 100.0,
            tubes: vec![],
            reserve: 5,
            solutions: vec![],
            solution_errors: vec![],
            torpedoes: vec![],
        }
    }

    #[test]
    fn applied_estimate_extrapolates_only_hypotheses_from_observation_origin() {
        let data = weapons();
        let expected = Vec2::new(100.0 + 8.0 * KNOTS_TO_METERS_PER_SECOND * 100.0, 1800.0);
        assert!(applied_position(&data).unwrap().distance(expected) < 0.001);
        let mut older = data.clone();
        older.visual_target_age += 10.0;
        assert!(
            (applied_position(&older).unwrap().x
                - expected.x
                - 8.0 * KNOTS_TO_METERS_PER_SECOND * 10.0)
                .abs()
                < 0.001
        );
        let mut replaced = data.clone();
        replaced.visual_target.as_mut().unwrap().distance = 1_000.0;
        assert!((applied_position(&replaced).unwrap().y - 800.0).abs() < 0.001);
    }

    #[test]
    fn no_applied_designation_means_no_estimated_position() {
        let mut data = weapons();
        data.visual_target = None;
        assert!(applied_position(&data).is_none());
        data = weapons();
        data.visual_target_age = f32::NAN;
        assert!(applied_position(&data).is_none());
    }

    #[test]
    fn old_estimate_remains_drawable_after_contact_expiration_and_is_labelled() {
        let mut data = weapons();
        data.visual_target_age = 300.0;
        let old = estimate_is_old(data.visual_target_age, PeriscopeConfig::default());
        assert!(old);
        assert!(applied_position(&data).is_some());
        let label = estimate_label(
            data.visual_target.unwrap(),
            data.visual_target_age,
            old,
            false,
        );
        assert!(label.contains("V7 EST. r2") && label.contains("300s / ANCIEN"));
        assert!(label.contains("HYPOTHESES MANUELLES"));
        let compact = estimate_label(
            data.visual_target.unwrap(),
            data.visual_target_age,
            old,
            true,
        );
        assert!(compact.contains("V7 EST. r2 / 300s") && compact.contains("ANCIEN / HYP. MAN."));
        assert!(!estimate_is_old(29.9, PeriscopeConfig::default()));
        assert!(estimate_is_old(30.0, PeriscopeConfig::default()));
    }

    #[test]
    fn bearing_ray_stays_anchored_to_frozen_observation_and_never_supplies_a_range() {
        let contact = VisualContact {
            id: VisualContactId(1),
            bearing: 0.0,
            kind: SilhouetteKind::SingleFunnelCargo,
            observed_at: 10.0,
            age: 31.0,
            old: true,
            shared: true,
            origin_x: 250.0,
            origin_y: -400.0,
        };
        let mut view = ChartView::default();
        let (a, b) = observed_ray(&view, &contact).unwrap();
        assert!(view.unproject(a).distance(Vec2::new(250.0, -400.0)) < 0.01);
        assert!((a.x - b.x).abs() < 0.01 && b.y < a.y);
        view.center = Vec2::new(800.0, -1000.0);
        view.zoom(0.5);
        let (moved, _) = observed_ray(&view, &contact).unwrap();
        assert!(view.unproject(moved).distance(Vec2::new(250.0, -400.0)) < 0.01);
    }
}
