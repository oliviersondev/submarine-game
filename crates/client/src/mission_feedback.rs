//! Non-blocking feedback derived only from the current authoritative snapshot.
use shared::{MissionPhase, MissionProgress};

pub(crate) fn extraction_notice(
    mission: Option<MissionProgress>,
    x: f32,
    y: f32,
) -> Option<&'static str> {
    let mission = mission?;
    let zone = mission.briefing.extraction;
    (mission.phase == MissionPhase::Attack && (x - zone.x).hypot(y - zone.y) <= zone.radius)
        .then_some(
        "ZONE D'EXTRACTION ATTEINTE\nOBJECTIF PRINCIPAL NON ACCOMPLI\nMISSION TOUJOURS EN COURS",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{MissionBriefing, MissionDefeat};

    fn mission(phase: MissionPhase) -> MissionProgress {
        MissionProgress {
            briefing: MissionBriefing::level_one(0),
            phase,
            elapsed_seconds: 120.0,
        }
    }

    #[test]
    fn notice_is_persistent_inside_and_includes_the_zone_boundary() {
        let progress = mission(MissionPhase::Attack);
        let zone = progress.briefing.extraction;
        for (x, y) in [
            (zone.x, zone.y),
            (zone.x + zone.radius, zone.y),
            (zone.x, zone.y - zone.radius),
        ] {
            let text = extraction_notice(Some(progress), x, y).unwrap();
            assert!(text.contains("ZONE D'EXTRACTION ATTEINTE"));
            assert!(text.contains("OBJECTIF PRINCIPAL NON ACCOMPLI"));
            assert!(text.contains("MISSION TOUJOURS EN COURS"));
            assert_eq!(extraction_notice(Some(progress), x, y), Some(text));
        }
    }

    #[test]
    fn leaving_or_completing_objective_removes_the_notice() {
        let progress = mission(MissionPhase::Attack);
        let zone = progress.briefing.extraction;
        assert!(extraction_notice(Some(progress), zone.x + zone.radius + 0.1, zone.y).is_none());
        assert!(extraction_notice(Some(progress), zone.x, zone.y - zone.radius - 0.1).is_none());
        for phase in [
            MissionPhase::Extract,
            MissionPhase::Victory,
            MissionPhase::Defeat(MissionDefeat::SubmarineLost),
            MissionPhase::Defeat(MissionDefeat::TargetEscaped),
        ] {
            assert!(extraction_notice(Some(mission(phase)), zone.x, zone.y).is_none());
        }
        assert!(extraction_notice(None, zone.x, zone.y).is_none());
    }
}
