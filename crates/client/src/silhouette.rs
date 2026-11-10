//! Original, normalized profiles shared by the briefing and periscope renderer.
use shared::SilhouetteKind;

/// Left, bottom, width, height in a unit bounding box. Bow is on the right.
pub(crate) fn parts(kind: SilhouetteKind) -> [(f32, f32, f32, f32); 8] {
    match kind {
        SilhouetteKind::SingleFunnelCargo => [
            (0.0, 0.0, 1.0, 0.25),
            (0.10, 0.25, 0.20, 0.30),  // aft castle
            (0.16, 0.55, 0.08, 0.26),  // single funnel
            (0.43, 0.25, 0.018, 0.75), // two tall cargo masts
            (0.76, 0.25, 0.018, 0.75),
            (0.43, 0.70, 0.18, 0.035),
            (0.76, 0.70, 0.15, 0.035),
            (0.38, 0.25, 0.50, 0.08),
        ],
        SilhouetteKind::TwinFunnelCargo => [
            (0.0, 0.0, 1.0, 0.25),
            (0.38, 0.25, 0.32, 0.30), // central castle
            (0.43, 0.55, 0.075, 0.35),
            (0.58, 0.55, 0.075, 0.35),
            (0.17, 0.25, 0.018, 0.50), // short fore/aft masts
            (0.83, 0.25, 0.018, 0.50),
            (0.10, 0.25, 0.22, 0.12),
            (0.74, 0.25, 0.18, 0.12),
        ],
        SilhouetteKind::Escort => [
            (0.0, 0.0, 1.0, 0.18),
            (0.58, 0.18, 0.22, 0.27),
            (0.39, 0.18, 0.08, 0.35),
            (0.63, 0.45, 0.02, 0.55),
            (0.12, 0.18, 0.15, 0.10),
            (0.82, 0.18, 0.10, 0.10),
            (0.12, 0.25, 0.22, 0.025),
            (0.82, 0.25, 0.16, 0.025),
        ],
    }
}

pub(crate) fn description(kind: SilhouetteKind) -> &'static str {
    match kind {
        SilhouetteKind::SingleFunnelCargo => {
            "Cargo a chateau arriere, UNE cheminee et DEUX grands mats de charge."
        }
        SilhouetteKind::TwinFunnelCargo => {
            "Cargo a chateau central, DEUX cheminees rapprochees et mats courts aux extremites."
        }
        SilhouetteKind::Escort => "Escorte basse, passerelle avant et mat eleve.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_profiles_are_distinct_and_fit_their_bounds() {
        let a = parts(SilhouetteKind::SingleFunnelCargo);
        let b = parts(SilhouetteKind::TwinFunnelCargo);
        assert_ne!(a, b);
        for (x, y, w, h) in a.into_iter().chain(b).chain(parts(SilhouetteKind::Escort)) {
            assert!(x >= 0.0 && y >= 0.0 && w > 0.0 && h > 0.0);
            assert!(x + w <= 1.0 && y + h <= 1.0);
        }
    }
}
