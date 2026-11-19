//! Visual observations and transmitted hypotheses, deliberately separate from sonar tracks.
use bevy::prelude::*;

use crate::{
    chart::ChartView,
    chart_ui::{set_line, ChartViewSet},
    network::{GameState, NetworkReceiveSet},
    visual_chart,
};

// Same bounded contact history as the authoritative periscope.
const CONTACT_SLOTS: usize = 64;

#[derive(Component, Clone, Copy)]
enum Mark {
    Ray(usize),
    RayLabel(usize),
    Estimate,
    EstimateLabel,
}

pub struct VisualChartUiPlugin;
impl Plugin for VisualChartUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, paint.after(NetworkReceiveSet).after(ChartViewSet));
    }
}

pub(crate) fn spawn(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
            ZIndex(4),
        ))
        .with_children(|overlay| {
            for mark in (0..CONTACT_SLOTS)
                .flat_map(|i| [Mark::Ray(i), Mark::RayLabel(i)])
                .chain([Mark::Estimate, Mark::EstimateLabel])
            {
                overlay.spawn((
                    mark,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.2, 0.95, 0.95)),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    UiTransform::default(),
                    BackgroundColor(Color::NONE),
                    BorderColor::all(Color::NONE),
                    Pickable::IGNORE,
                    Visibility::Hidden,
                ));
            }
        });
}

fn label_position(view: &ChartView, point: Vec2, text: &str, below: bool) -> (Vec2, f32) {
    // Monospaced 10px labels: size to content, not an unnecessarily wide fixed box.
    let width = (text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f32
        * 6.0
        + 4.0)
        .min(225.0)
        .min((view.size.x - 8.0).max(1.0));
    let height = text.lines().count() as f32 * 12.0;
    let x = if point.x + 12.0 + width <= view.size.x - 4.0 {
        point.x + 12.0
    } else if point.x - 12.0 - width >= 4.0 {
        point.x - 12.0 - width
    } else {
        (view.size.x - width) * 0.5
    };
    let y = if below {
        point.y + 12.0
    } else {
        point.y - height - 4.0
    };
    (
        Vec2::new(
            x.max(4.0),
            y.clamp(18.0, (view.size.y - height - 4.0).max(18.0)),
        ),
        width,
    )
}

fn paint(
    state: Res<GameState>,
    view: Res<ChartView>,
    mut marks: Query<(
        &Mark,
        &mut Text,
        &mut TextColor,
        &mut Node,
        &mut UiTransform,
        &mut BackgroundColor,
        &mut BorderColor,
        &mut Visibility,
    )>,
) {
    if !state.is_changed() && !view.is_changed() {
        return;
    }
    for (
        mark,
        mut text,
        mut text_color,
        mut node,
        mut transform,
        mut background,
        mut border,
        mut visibility,
    ) in &mut marks
    {
        *visibility = Visibility::Hidden;
        text.0.clear();
        node.width = Val::Auto;
        node.height = Val::Auto;
        node.border = UiRect::ZERO;
        transform.rotation = Rot2::IDENTITY;
        background.0 = Color::NONE;
        *border = BorderColor::all(Color::NONE);
        if !state.game_started || state.submarine.is_none() {
            continue;
        }
        let old;
        match *mark {
            Mark::Ray(i) | Mark::RayLabel(i) => {
                let Some(contact) = state.visual_contacts.get(i) else {
                    continue;
                };
                let Some((a, b)) = visual_chart::observed_ray(&view, contact) else {
                    continue;
                };
                old = contact.old;
                if matches!(mark, Mark::Ray(_)) {
                    set_line(&mut node, &mut transform, a, b, 1.0);
                    background.0 = if old {
                        Color::srgba(0.35, 0.65, 0.70, 0.25)
                    } else {
                        Color::srgba(0.2, 0.95, 0.95, 0.45)
                    };
                } else {
                    text.0 = format!(
                        "V{} OBS. / {:.0}s{} / DIST. ?",
                        contact.id.0,
                        contact.age,
                        if old { " / ANCIEN" } else { "" }
                    );
                    let (position, width) = label_position(&view, a.lerp(b, 0.75), &text.0, false);
                    let same_estimate_visible = state.weapons.as_ref().is_some_and(|w| {
                        w.visual_target.is_some_and(|t| t.contact == contact.id)
                            && visual_chart::applied_position(w)
                                .is_some_and(|p| view.contains(view.project(p)))
                    });
                    // In compact charts keep the observation header above the estimate caption.
                    // Very short charts retain the ray but avoid a second overlapping caption.
                    let position = if view.size.x < 500.0 && same_estimate_visible {
                        if view.size.y < 80.0 {
                            continue;
                        }
                        Vec2::new(4.0, 18.0)
                    } else {
                        position
                    };
                    node.left = px(position.x);
                    node.top = px(position.y);
                    node.width = px(width);
                }
            }
            Mark::Estimate | Mark::EstimateLabel => {
                // Never read the Captain's local form or any sonar track to create this marker.
                let Some(weapons) = state.weapons.as_ref() else {
                    continue;
                };
                let Some(target) = weapons.visual_target else {
                    continue;
                };
                let Some(world) = visual_chart::applied_position(weapons) else {
                    continue;
                };
                let point = view.project(world);
                if !view.contains(point) {
                    continue;
                }
                old = visual_chart::estimate_is_old(
                    weapons.visual_target_age,
                    state.periscope_config.unwrap_or_default(),
                );
                if matches!(mark, Mark::Estimate) {
                    node.left = px(point.x - 6.0);
                    node.top = px(point.y - 6.0);
                    node.width = px(12);
                    node.height = px(12);
                    node.border = UiRect::all(px(2));
                    transform.rotation = Rot2::degrees(45.0);
                    *border = BorderColor::all(if old {
                        Color::srgb(0.35, 0.65, 0.70)
                    } else {
                        Color::srgb(0.2, 0.95, 0.95)
                    });
                } else {
                    text.0 = visual_chart::estimate_label(
                        target,
                        weapons.visual_target_age,
                        old,
                        view.size.x < 500.0 || view.size.y < 140.0,
                    );
                    let (mut position, width) = label_position(&view, point, &text.0, true);
                    if view.size.x < 500.0
                        && view.size.y >= 80.0
                        && state.visual_contacts.iter().any(|c| c.id == target.contact)
                    {
                        position.y = position.y.max(34.0);
                    }
                    node.left = px(position.x);
                    node.top = px(position.y);
                    node.width = px(width);
                }
            }
        }
        text_color.0 = if old {
            Color::srgb(0.40, 0.70, 0.75)
        } else {
            Color::srgb(0.2, 0.95, 0.95)
        };
        *visibility = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn observation_is_only_a_ray_and_applied_estimate_survives_fresh_observations_and_expiry() {
        let mut world = World::new();
        world
            .run_system_once(|mut commands: Commands| {
                commands.spawn(Node::default()).with_children(spawn);
            })
            .unwrap();
        world.insert_resource(ChartView::default());
        let mut state = GameState::default();
        state.game_started = true;
        state.submarine = Some(shared::SubmarineSnapshot {
            common: shared::CommonMeasurements {
                x: 0.0,
                y: 0.0,
                heading: 0.0,
                speed: 0.0,
                depth: 12.0,
                dive_state: shared::DiveState::Periscope,
                acoustic_level: shared::AcousticLevel::Low,
                alerts: vec![],
                damage: shared::DamageSummary::default(),
            },
            pilot: None,
            engineering: None,
        });
        state.visual_contacts.push(shared::VisualContact {
            id: shared::VisualContactId(1),
            bearing: 0.0,
            kind: shared::SilhouetteKind::SingleFunnelCargo,
            observed_at: 10.0,
            age: 0.0,
            old: false,
            shared: false,
            origin_x: 100.0,
            origin_y: -200.0,
        });
        state.weapons = Some(shared::WeaponsMeasurements {
            visual_target: None,
            visual_target_age: 0.0,
            tubes: vec![],
            reserve: 5,
            solutions: vec![],
            solution_errors: vec![],
            torpedoes: vec![],
        });
        world.insert_resource(state);
        world.run_system_once(paint).unwrap();
        let estimate = world
            .query::<(Entity, &Mark)>()
            .iter(&world)
            .find(|(_, mark)| matches!(mark, Mark::Estimate))
            .unwrap()
            .0;
        let label = world
            .query::<(Entity, &Mark)>()
            .iter(&world)
            .find(|(_, mark)| matches!(mark, Mark::EstimateLabel))
            .unwrap()
            .0;
        let ray = world
            .query::<(Entity, &Mark)>()
            .iter(&world)
            .find(|(_, mark)| matches!(mark, Mark::Ray(0)))
            .unwrap()
            .0;
        assert_eq!(
            *world.get::<Visibility>(ray).unwrap(),
            Visibility::Inherited
        );
        assert_eq!(
            *world.get::<Visibility>(estimate).unwrap(),
            Visibility::Hidden
        );
        {
            let mut state = world.resource_mut::<GameState>();
            let weapons = state.weapons.as_mut().unwrap();
            weapons.visual_target = Some(shared::VisualTargetEstimate {
                revision: 2,
                contact: shared::VisualContactId(1),
                kind: shared::SilhouetteKind::SingleFunnelCargo,
                bearing: 0.0,
                observed_at: 10.0,
                origin_x: 100.0,
                origin_y: -200.0,
                distance: 1000.0,
                heading: 90.0,
                speed: 8.0,
            });
            weapons.visual_target_age = 10.0;
        }
        world.run_system_once(paint).unwrap();
        assert_eq!(
            *world.get::<Visibility>(estimate).unwrap(),
            Visibility::Inherited
        );
        let ray_label = world
            .query::<(Entity, &Mark)>()
            .iter(&world)
            .find(|(_, mark)| matches!(mark, Mark::RayLabel(0)))
            .unwrap()
            .0;
        world.resource_mut::<ChartView>().size = Vec2::new(320.0, 96.0);
        world.run_system_once(paint).unwrap();
        let Val::Px(observed_top) = world.get::<Node>(ray_label).unwrap().top else {
            panic!("pixel label expected")
        };
        let Val::Px(estimated_top) = world.get::<Node>(label).unwrap().top else {
            panic!("pixel label expected")
        };
        assert!(
            estimated_top >= observed_top + 16.0,
            "compact captions overlap"
        );
        world.resource_mut::<ChartView>().size = Vec2::new(1280.0, 900.0);
        world.run_system_once(paint).unwrap();
        let before = world.get::<Node>(estimate).unwrap().clone();
        {
            let mut state = world.resource_mut::<GameState>();
            state.submarine.as_mut().unwrap().common.x = 5000.0;
            state.visual_contacts[0].bearing = 90.0;
            state.visual_contacts[0].origin_x = 5000.0;
            state.visual_contacts[0].observed_at = 20.0;
        }
        world.run_system_once(paint).unwrap();
        let after = world.get::<Node>(estimate).unwrap();
        assert_eq!((before.left, before.top), (after.left, after.top));
        {
            let mut state = world.resource_mut::<GameState>();
            state.visual_contacts.clear();
            state.weapons.as_mut().unwrap().visual_target_age = 300.0;
        }
        world.run_system_once(paint).unwrap();
        assert_eq!(*world.get::<Visibility>(ray).unwrap(), Visibility::Hidden);
        assert_eq!(
            *world.get::<Visibility>(estimate).unwrap(),
            Visibility::Inherited
        );
        let text = &world.get::<Text>(label).unwrap().0;
        assert!(text.contains("V1 EST. r2") && text.contains("300s / ANCIEN"));
        assert!(text.contains("HYPOTHESES MANUELLES"));
    }
}
