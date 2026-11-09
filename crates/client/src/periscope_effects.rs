use crate::{
    network::{GameState, NetworkReceiveSet},
    periscope_ui::{PeriscopeUi, View},
};
use bevy::prelude::*;

const LIFETIME: f64 = 2.0;
const SLOTS: usize = 16;

#[derive(Clone)]
struct Observation {
    bearing: f32,
    size: f32,
    expires: f64,
}

#[derive(Resource, Default)]
struct Effects {
    seen: u64,
    active: Vec<Observation>,
}

impl Effects {
    fn receive(&mut self, state: &GameState, watching: bool, now: f64) {
        if state.surface_explosion_sequence < self.seen {
            self.active.clear();
        }
        self.active.retain(|o| o.expires > now);
        if !watching {
            self.active.clear();
        } else {
            for &(id, bearing, size) in &state.surface_explosions {
                if id > self.seen && bearing.is_finite() && size.is_finite() && size > 0.0 {
                    if self.active.len() == SLOTS {
                        self.active.remove(0);
                    }
                    self.active.push(Observation {
                        bearing,
                        size,
                        expires: now + LIFETIME,
                    });
                }
            }
        }
        self.seen = state.surface_explosion_sequence;
    }
}

#[derive(Component)]
struct Splash(usize);
#[derive(Component)]
struct Part(usize);

pub struct PeriscopeEffectsPlugin;
impl Plugin for PeriscopeEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Effects>()
            .add_systems(Update, update.after(NetworkReceiveSet));
    }
}

pub(crate) fn spawn(parent: &mut ChildSpawnerCommands) {
    for slot in 0..SLOTS {
        parent
            .spawn((
                Splash(slot),
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Visibility::Hidden,
                Pickable::IGNORE,
                ZIndex(3),
            ))
            .with_children(|plume| {
                for (left, height) in [(5.0, 65.0), (40.0, 100.0), (75.0, 65.0)] {
                    plume.spawn((
                        Part(slot),
                        Node {
                            position_type: PositionType::Absolute,
                            left: percent(left),
                            bottom: px(0),
                            width: percent(20),
                            height: percent(height),
                            ..default()
                        },
                        BackgroundColor(Color::WHITE),
                        Pickable::IGNORE,
                    ));
                }
            });
    }
}

fn update(
    state: Res<GameState>,
    ui: Res<PeriscopeUi>,
    time: Res<Time<Real>>,
    mut effects: ResMut<Effects>,
    view: Single<&ComputedNode, With<View>>,
    mut splashes: Query<(&Splash, &mut Node, &mut Visibility)>,
    mut parts: Query<(&Part, &mut BackgroundColor)>,
) {
    let scope = state.periscope.as_ref();
    let watching = state.game_started && ui.open && scope.is_some_and(|p| p.raised);
    let now = time.elapsed_secs_f64();
    effects.receive(&state, watching, now);
    let width = view.size.x * view.inverse_scale_factor;
    let height = view.size.y * view.inverse_scale_factor;
    for (Splash(slot), mut node, mut visibility) in &mut splashes {
        *visibility = Visibility::Hidden;
        let (Some(o), Some(scope)) = (effects.active.get(*slot), scope) else {
            continue;
        };
        let delta = (o.bearing - scope.bearing + 180.0).rem_euclid(360.0) - 180.0;
        if !watching || delta.abs() > scope.field_of_view * 0.5 {
            continue;
        }
        let age = (LIFETIME - (o.expires - now)) as f32;
        let size = (o.size / scope.field_of_view * width).clamp(8.0, 70.0);
        let plume_height = size * (0.7 + (age / 0.2).min(1.0)) * (1.0 - age / 3.0);
        node.width = px(size);
        node.height = px(plume_height);
        node.left = px(width * (0.5 + delta / scope.field_of_view) - size * 0.5);
        node.top = px(height * 0.6 - plume_height);
        *visibility = Visibility::Inherited;
    }
    for (Part(slot), mut color) in &mut parts {
        let alpha = effects
            .active
            .get(*slot)
            .map_or(0.0, |o| ((o.expires - now) / LIFETIME) as f32);
        color.0 = Color::srgba(1.0, 0.93, 0.72, alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lowering_or_closed_view_discards_effects_without_retroactive_replay() {
        let mut state = GameState::default();
        state.surface_explosion_sequence = 1;
        state.surface_explosions.push_back((1, 0.0, 1.0));
        let mut effects = Effects::default();
        effects.receive(&state, true, 0.0);
        assert_eq!(effects.active.len(), 1);
        effects.receive(&state, false, 0.1);
        effects.receive(&state, true, 0.2);
        assert!(effects.active.is_empty());
        state.surface_explosion_sequence = 2;
        state.surface_explosions.push_back((2, 10.0, 1.0));
        effects.receive(&state, true, 0.3);
        assert_eq!(effects.active.len(), 1);
        effects.receive(&state, true, 2.3);
        assert!(effects.active.is_empty());
    }
}
