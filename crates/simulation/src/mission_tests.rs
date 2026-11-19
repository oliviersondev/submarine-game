use super::*;
use shared::{MissionDefeat, MissionPhase, ShotTarget, SilhouetteKind, VisualContactId};

fn sink(sim: &mut Simulation, index: usize) {
    let vessel = &sim.world.vessels[index];
    let (x, y) = (vessel.x, vessel.y);
    let config = shared::WeaponsConfig {
        damage: 100.0,
        ..sim.config.weapons
    };
    assert_eq!(sim.world.impact(x, y, x, y, 2.0, config), Some(true));
}

#[test]
fn level_one_has_unique_cargos_and_briefing_matches_private_objective_for_every_seed() {
    for seed in 0..100 {
        let sim = Simulation::with_config(MissionConfig::new(seed));
        assert_eq!(sim.world.vessels.len(), 3);
        let a = sim.world.vessels[0].silhouette(0);
        let b = sim.world.vessels[1].silhouette(1);
        assert_ne!(a, b);
        assert_ne!(a, SilhouetteKind::Escort);
        assert_ne!(b, SilhouetteKind::Escort);
        assert_eq!(sim.world.vessels[2].silhouette(2), SilhouetteKind::Escort);
        assert_eq!(
            sim.mission.briefing.target,
            sim.world.vessels[(seed % 2) as usize].silhouette((seed % 2) as usize)
        );
    }
}

#[test]
fn wrong_cargo_never_completes_objective_or_confirms_kill() {
    let mut sim = Simulation::new();
    sink(&mut sim, 1);
    let mut events = vec![];
    sim.update_mission(&mut events);
    assert_eq!(sim.mission.phase, MissionPhase::Attack);
    assert!(events.is_empty());
    let zone = sim.mission.briefing.extraction;
    sim.state.x = zone.x;
    sim.state.y = zone.y;
    sim.update_mission(&mut events);
    assert_eq!(sim.mission.phase, MissionPhase::Attack);
}

#[test]
fn objective_is_announced_once_even_unobserved_and_survives_missed_events() {
    let mut sim = Simulation::new();
    assert!(!sim.periscope_measurements().raised);
    sink(&mut sim, 0);
    let mut events = vec![];
    sim.update_mission(&mut events);
    sim.update_mission(&mut events);
    assert_eq!(events, vec![GameEvent::ObjectiveDestroyed]);
    assert_eq!(sim.mission_progress().phase, MissionPhase::Extract);
    sim.world.vessels[1].y = -30_000.0;
    sim.update_mission(&mut events);
    assert_eq!(sim.mission.phase, MissionPhase::Extract);
}

#[test]
fn extraction_boundary_is_inclusive_and_pursuit_does_not_prevent_victory() {
    let mut sim = Simulation::new();
    sink(&mut sim, 0);
    sim.world.escort_ai.observe(crate::enemy_ai::Observation {
        x: sim.state.x,
        y: sim.state.y,
        uncertainty: 50.0,
        confidence: 100.0,
        tick: 0,
    });
    let zone = sim.mission.briefing.extraction;
    sim.state.x = zone.x + zone.radius + 0.1;
    sim.state.y = zone.y;
    sim.update_mission(&mut vec![]);
    assert_eq!(sim.mission.phase, MissionPhase::Extract);
    sim.state.x = zone.x + zone.radius;
    sim.update_mission(&mut vec![]);
    assert_eq!(sim.mission.phase, MissionPhase::Victory);
}

#[test]
fn living_target_escaping_on_any_axis_is_a_defeat_but_boundary_is_inside() {
    for (x, y) in [
        (7_000.1, 0.0),
        (-7_000.1, 0.0),
        (0.0, 7_000.1),
        (0.0, -7_000.1),
    ] {
        let mut sim = Simulation::new();
        sim.world.vessels[0].x = 7_000.0;
        sim.world.vessels[0].y = -7_000.0;
        sim.update_mission(&mut vec![]);
        assert_eq!(sim.mission.phase, MissionPhase::Attack);
        sim.world.vessels[0].x = x;
        sim.world.vessels[0].y = y;
        sim.update_mission(&mut vec![]);
        assert_eq!(
            sim.mission.phase,
            MissionPhase::Defeat(MissionDefeat::TargetEscaped)
        );
    }
}

#[test]
fn own_loss_wins_over_objective_and_extraction_and_terminal_state_is_immutable() {
    let mut sim = Simulation::new();
    sink(&mut sim, 0);
    sim.state.x = sim.mission.briefing.extraction.x;
    sim.state.y = sim.mission.briefing.extraction.y;
    sim.damage.hit(
        &mut sim.state.hull_integrity,
        100.0,
        0,
        sim.tick,
        sim.config.damage,
    );
    sim.update_mission(&mut vec![]);
    assert_eq!(
        sim.mission.phase,
        MissionPhase::Defeat(MissionDefeat::SubmarineLost)
    );
    let before = sim.state.clone();
    let progress = sim.mission_progress();
    assert!(sim.tick(0.05).is_empty());
    sim.automate_engineer();
    sim.automate_sonar();
    assert!(sim.automate_weapons(true, true).is_empty());
    assert_eq!(sim.state, before);
    assert_eq!(sim.mission_progress(), progress);
}

#[test]
fn commands_and_bots_cannot_mutate_a_finished_mission() {
    let mut sim = Simulation::new();
    sink(&mut sim, 0);
    sim.state.y = sim.mission.briefing.extraction.y;
    sim.update_mission(&mut vec![]);
    assert_eq!(sim.mission.phase, MissionPhase::Victory);
    assert_eq!(
        sim.apply_command(PlayerCommand::SetSpeed(5.0)),
        Err(SimulationError::MissionFinished)
    );
    assert_eq!(
        sim.apply_pilot_order(PilotOrder {
            heading: 180.0,
            speed: 5.0,
            depth: 12.0
        }),
        Err(SimulationError::MissionFinished)
    );
    let state = sim.state.clone();
    let weapons = sim.weapons_measurements();
    sim.automate_engineer();
    sim.automate_sonar();
    assert!(sim.automate_weapons(true, true).is_empty());
    assert!(sim.tick(1.0).is_empty());
    assert_eq!(sim.state, state);
    assert_eq!(sim.weapons_measurements(), weapons);
}

#[test]
fn visual_torpedo_bot_attack_then_pilot_extraction_is_deterministic() {
    fn run() -> shared::MissionProgress {
        let mut sim = Simulation::new();
        // Controlled range fixture: real tube procedures, flight and navigation.
        sim.world.vessels[0].x = 0.0;
        sim.world.vessels[0].y = 1_000.0;
        sim.world.vessels[0].speed = 0.0;
        sim.world.vessels[2].x = 30_000.0;
        sim.apply_command(PlayerCommand::SetPeriscopeRaised(true))
            .unwrap();
        let token = sim.periscope_measurements().silhouettes[0].observation_id;
        sim.apply_command(PlayerCommand::CreateVisualContact {
            observation_id: token,
        })
        .unwrap();
        sim.apply_command(PlayerCommand::DesignateVisualTarget {
            contact: VisualContactId(1),
            distance: 1_000.0,
            heading: 0.0,
            speed: 0.0,
        })
        .unwrap();
        sim.apply_command(PlayerCommand::SetPeriscopeRaised(false))
            .unwrap();
        sim.apply_command(PlayerCommand::OrderWeaponsBot {
            target: ShotTarget::Visual(1),
        })
        .unwrap();
        let mut announced = 0;
        for _ in 0..3_000 {
            sim.automate_engineer();
            announced += sim
                .tick(0.05)
                .iter()
                .filter(|e| **e == GameEvent::ObjectiveDestroyed)
                .count();
            sim.automate_sonar();
            sim.automate_weapons(true, false);
            let tube = &sim.weapons_measurements().tubes[0];
            if tube.phase == shared::TubePhase::Ready && !tube.authorized {
                sim.apply_command(PlayerCommand::AuthorizeShot {
                    tube: 0,
                    settings: tube.settings.unwrap(),
                })
                .unwrap();
            }
            if sim.mission.phase == MissionPhase::Extract {
                break;
            }
        }
        assert_eq!(announced, 1);
        assert_eq!(sim.mission.phase, MissionPhase::Extract);
        sim.apply_pilot_order(PilotOrder {
            heading: 180.0,
            speed: 5.0,
            depth: 12.0,
        })
        .unwrap();
        for _ in 0..20_000 {
            sim.automate_engineer();
            sim.tick(0.05);
            if sim.mission.phase.finished() {
                break;
            }
        }
        assert_eq!(sim.mission.phase, MissionPhase::Victory);
        sim.mission_progress()
    }
    assert_eq!(run(), run());
}
