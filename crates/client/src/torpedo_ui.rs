//! Tactical overlays use only fire-control hypotheses and own estimated torpedo runs.
use bevy::prelude::*;

use crate::{
    network::{GameState, NetworkReceiveSet},
    weapons_ui::WeaponUi,
};

const PREVIEW_DOTS: usize = 24;
const TRAIL_DOTS: usize = 6;
const RUN_SLOTS: usize = 5;
const MAP_RANGE: f32 = 10_000.0;

#[derive(Component, Clone, Copy)]
enum Mark {
    Preview(usize),
    Intercept,
    Trail(usize, usize),
    Torpedo(usize),
    Legend,
}

#[derive(Resource, Default)]
struct SnapshotClock {
    snapshot: u64,
    received_at: f64,
}

pub struct TorpedoUiPlugin;
impl Plugin for TorpedoUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SnapshotClock>()
            .add_systems(Update, update.after(NetworkReceiveSet));
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
            ZIndex(2),
        ))
        .with_children(|overlay| {
            for mark in (0..PREVIEW_DOTS)
                .map(Mark::Preview)
                .chain([Mark::Intercept, Mark::Legend])
                .chain((0..RUN_SLOTS).flat_map(|slot| {
                    (0..TRAIL_DOTS)
                        .map(move |i| Mark::Trail(slot, i))
                        .chain([Mark::Torpedo(slot)])
                }))
            {
                overlay.spawn((
                    mark,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(match mark {
                        Mark::Preview(_) | Mark::Intercept => Color::srgb(0.45, 0.75, 0.8),
                        _ => Color::srgb(1.0, 0.85, 0.3),
                    }),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    Pickable::IGNORE,
                    Visibility::Hidden,
                ));
            }
        });
}

/// Same north-up, 10km radius as the tactical contact map. No guessed enemy position.
fn map_point(relative: Vec2) -> Vec2 {
    let point = relative.clamp_length_max(MAP_RANGE) / MAP_RANGE * 40.0;
    Vec2::new(50.0 + point.x, 50.0 - point.y)
}

fn direction(bearing: f32) -> Vec2 {
    let radians = bearing.to_radians();
    Vec2::new(radians.sin(), radians.cos())
}

fn update(
    state: Res<GameState>,
    ui: Res<WeaponUi>,
    time: Res<Time<Real>>,
    mut clock: ResMut<SnapshotClock>,
    mut marks: Query<(&Mark, &mut Text, &mut Node, &mut Visibility)>,
) {
    if state.snapshot_id != clock.snapshot {
        clock.snapshot = state.snapshot_id;
        clock.received_at = time.elapsed_secs_f64();
    }
    // Extrapolate at most one snapshot interval, never invent movement during a disconnection.
    let elapsed = ((time.elapsed_secs_f64() - clock.received_at) as f32).clamp(0.0, 0.05);
    let own = state
        .submarine
        .as_ref()
        .map(|s| Vec2::new(s.common.x, s.common.y));
    let speed = state.weapons_config.unwrap_or_default().torpedo_speed_knots * 0.514444;
    for (mark, mut text, mut node, mut visibility) in &mut marks {
        *visibility = Visibility::Hidden;
        let (Some(weapons), Some(own)) = (state.weapons.as_ref(), own) else {
            continue;
        };
        if !state.game_started {
            continue;
        }
        let position = match *mark {
            Mark::Legend => {
                text.0 = format!("ESTIME / tube {} / + interception / 10km", ui.tube + 1);
                Vec2::new(1.0, 88.0)
            }
            Mark::Preview(_) | Mark::Intercept => {
                let Some(solution) = weapons.solutions.get(ui.tube).and_then(|s| s.as_ref()) else {
                    continue;
                };
                let end = direction(solution.bearing) * speed * solution.run_seconds;
                let fraction = if let Mark::Preview(i) = mark {
                    (*i as f32 + 1.0) / (PREVIEW_DOTS as f32 + 1.0)
                } else {
                    1.0
                };
                text.0 = if matches!(mark, Mark::Intercept) {
                    "+"
                } else {
                    "."
                }
                .into();
                map_point(end * fraction)
            }
            Mark::Trail(slot, _) | Mark::Torpedo(slot) => {
                let Some(t) = weapons.torpedoes.get(slot) else {
                    continue;
                };
                let dir = direction(t.bearing);
                let advance = (speed * elapsed).min(t.remaining_meters);
                let mut point = Vec2::new(t.x, t.y) + dir * advance;
                if let Mark::Trail(_, i) = mark {
                    let travelled = point.distance(Vec2::new(t.launch_x, t.launch_y));
                    let behind = ((i + 1) as f32 * 80.0).min(travelled);
                    point -= dir * behind;
                    text.0 = ".".into();
                } else {
                    text.0 = format!("o TOR{}", t.id);
                }
                map_point(point - own)
            }
        };
        node.left = percent(position.x);
        node.top = percent(position.y);
        *visibility = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tactical_geometry_is_north_up_and_bounded() {
        assert_eq!(map_point(Vec2::ZERO), Vec2::splat(50.0));
        assert_eq!(map_point(Vec2::new(0.0, 10_000.0)), Vec2::new(50.0, 10.0));
        assert_eq!(map_point(Vec2::new(20_000.0, 0.0)), Vec2::new(90.0, 50.0));
        assert!(direction(90.0).distance(Vec2::X) < 0.001);
    }
}
