//! Ordered pointer input, independent of frame-sampled Interaction/hover maps.
use std::collections::HashMap;

use bevy::{
    camera::RenderTarget,
    picking::{
        pointer::{Location, PointerAction, PointerId, PointerInput},
        PickingSystems,
    },
    prelude::*,
    ui::{clip_check_recursive, ComputedUiTargetCamera, OverrideClip, UiGlobalTransform, UiStack},
    window::PrimaryWindow,
};

const DRAG_THRESHOLD: f32 = 8.0;

#[derive(Message, Clone, Copy, Debug)]
pub(crate) struct ButtonActivated(pub Entity);

#[derive(Component)]
struct ActivateOnClick;

#[derive(Clone)]
struct PressGesture {
    button: Entity,
    origin: Location,
    dragged: bool,
}

impl PressGesture {
    fn moved_to(&mut self, location: &Location) {
        self.dragged |= self.origin.target != location.target
            || self.origin.position.distance_squared(location.position)
                >= DRAG_THRESHOLD * DRAG_THRESHOLD;
    }
}

#[derive(Resource, Default)]
struct Gestures(HashMap<PointerId, PressGesture>);

impl Gestures {
    fn input(
        &mut self,
        event: &PointerInput,
        hit: impl Fn(&Location) -> Option<Entity>,
    ) -> Option<Entity> {
        match event.action {
            PointerAction::Press(PointerButton::Primary) => {
                self.0.remove(&event.pointer_id);
                if let Some(button) = hit(&event.location) {
                    self.0.insert(
                        event.pointer_id,
                        PressGesture {
                            button,
                            origin: event.location.clone(),
                            dragged: false,
                        },
                    );
                }
            }
            PointerAction::Move { .. } => {
                if let Some(gesture) = self.0.get_mut(&event.pointer_id) {
                    gesture.moved_to(&event.location);
                }
            }
            PointerAction::Release(PointerButton::Primary) => {
                if let Some(mut gesture) = self.0.remove(&event.pointer_id) {
                    gesture.moved_to(&event.location);
                    if !gesture.dragged && hit(&event.location) == Some(gesture.button) {
                        return Some(gesture.button);
                    }
                }
            }
            PointerAction::Cancel => {
                self.0.remove(&event.pointer_id);
            }
            _ => {}
        }
        None
    }
}

pub(crate) struct ButtonActivationPlugin;
impl Plugin for ButtonActivationPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PointerInput>()
            .add_message::<ButtonActivated>()
            .init_resource::<Gestures>()
            .add_systems(PreUpdate, activate.after(PickingSystems::Last));
    }
}

pub(crate) fn enable(button: &mut EntityCommands) {
    button.insert(ActivateOnClick);
}

fn activate(
    mut input: MessageReader<PointerInput>,
    mut gestures: ResMut<Gestures>,
    mut activated: MessageWriter<ButtonActivated>,
    stack: Option<Res<UiStack>>,
    nodes: Query<(
        &ComputedNode,
        &UiGlobalTransform,
        &InheritedVisibility,
        &ComputedUiTargetCamera,
        Option<&Pickable>,
        Has<ActivateOnClick>,
    )>,
    clipping: Query<(&ComputedNode, &UiGlobalTransform, &Node)>,
    parents: Query<&ChildOf, Without<OverrideClip>>,
    cameras: Query<(&Camera, &RenderTarget)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
) {
    let Some(stack) = stack else {
        input.clear();
        gestures.0.clear();
        return;
    };
    // Match Bevy UI's physical coordinates, clipping, camera target and stacking.
    // Resolve separately for each input position, not once per frame: a touch
    // may begin AND end before the next hover map has ever contained it.
    let hit = |location: &Location| {
        for range in stack.partition.iter().rev() {
            for &entity in stack.uinodes[range.clone()].iter().rev() {
                let Ok((node, transform, visible, target, pickable, enabled)) = nodes.get(entity)
                else {
                    continue;
                };
                if !visible.get() || node.size() == Vec2::ZERO {
                    continue;
                }
                let Some(camera) = target.get() else {
                    continue;
                };
                let Ok((camera, render_target)) = cameras.get(camera) else {
                    continue;
                };
                if render_target
                    .normalize(primary_window.single().ok())
                    .as_ref()
                    != Some(&location.target)
                {
                    continue;
                }
                let mut position =
                    location.position * camera.target_scaling_factor().unwrap_or(1.0);
                if let Some(viewport) = camera.physical_viewport_rect() {
                    if !viewport.as_rect().contains(position) {
                        continue;
                    }
                    position -= viewport.min.as_vec2();
                }
                if !node.contains_point(*transform, position)
                    || !clip_check_recursive(position, entity, &clipping, &parents)
                {
                    continue;
                }
                let hoverable = pickable.is_none_or(|p| p.is_hoverable);
                if enabled && hoverable {
                    return Some(entity);
                }
                if pickable.is_none_or(|p| p.should_block_lower) {
                    return None;
                }
            }
        }
        None
    };
    for event in input.read() {
        if let Some(entity) = gestures.input(event, hit) {
            activated.write(ButtonActivated(entity));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::NormalizedRenderTarget;

    fn input(id: PointerId, position: Vec2, action: PointerAction) -> PointerInput {
        PointerInput::new(
            id,
            Location {
                position,
                target: NormalizedRenderTarget::None {
                    width: 1280,
                    height: 900,
                },
            },
            action,
        )
    }

    #[test]
    fn complete_mouse_clicks_and_touch_taps_in_one_frame_are_all_preserved() {
        let mut gestures = Gestures::default();
        let button = Entity::PLACEHOLDER;
        let mut count = 0;
        for pointer in [PointerId::Mouse, PointerId::Touch(1)] {
            for _ in 0..20 {
                for action in [
                    PointerAction::Press(PointerButton::Primary),
                    PointerAction::Release(PointerButton::Primary),
                ] {
                    count += gestures
                        .input(&input(pointer, Vec2::ZERO, action), |_| Some(button))
                        .is_some() as usize;
                }
            }
        }
        assert_eq!(count, 40);
        assert!(gestures.0.is_empty());
    }

    #[test]
    fn a_hold_is_not_an_activation_and_one_release_cannot_repeat() {
        let mut gestures = Gestures::default();
        let hit = |_: &Location| Some(Entity::PLACEHOLDER);
        assert!(gestures
            .input(
                &input(
                    PointerId::Mouse,
                    Vec2::ZERO,
                    PointerAction::Press(PointerButton::Primary)
                ),
                hit
            )
            .is_none());
        for _ in 0..30 {
            assert!(gestures
                .input(
                    &input(
                        PointerId::Mouse,
                        Vec2::ZERO,
                        PointerAction::Move { delta: Vec2::ZERO }
                    ),
                    hit
                )
                .is_none());
        }
        let release = input(
            PointerId::Mouse,
            Vec2::ZERO,
            PointerAction::Release(PointerButton::Primary),
        );
        assert_eq!(gestures.input(&release, hit), Some(Entity::PLACEHOLDER));
        assert!(gestures.input(&release, hit).is_none());
    }

    #[test]
    fn drag_cannot_click_even_after_returning_to_origin_and_jitter_is_allowed() {
        for (distance, accepted) in [(3.0, true), (8.0, false), (30.0, false)] {
            let mut gestures = Gestures::default();
            let hit = |_: &Location| Some(Entity::PLACEHOLDER);
            gestures.input(
                &input(
                    PointerId::Touch(1),
                    Vec2::ZERO,
                    PointerAction::Press(PointerButton::Primary),
                ),
                hit,
            );
            gestures.input(
                &input(
                    PointerId::Touch(1),
                    Vec2::new(0.0, distance),
                    PointerAction::Move {
                        delta: Vec2::new(0.0, distance),
                    },
                ),
                hit,
            );
            assert_eq!(
                gestures
                    .input(
                        &input(
                            PointerId::Touch(1),
                            Vec2::ZERO,
                            PointerAction::Release(PointerButton::Primary)
                        ),
                        hit
                    )
                    .is_some(),
                accepted
            );
        }
    }

    #[test]
    fn changing_target_cancelling_and_secondary_buttons_do_not_activate() {
        let mut gestures = Gestures::default();
        let hit = |_: &Location| Some(Entity::PLACEHOLDER);
        gestures.input(
            &input(
                PointerId::Mouse,
                Vec2::ZERO,
                PointerAction::Press(PointerButton::Primary),
            ),
            hit,
        );
        assert!(gestures
            .input(
                &input(
                    PointerId::Mouse,
                    Vec2::ZERO,
                    PointerAction::Release(PointerButton::Primary)
                ),
                |_| None
            )
            .is_none());
        gestures.input(
            &input(
                PointerId::Mouse,
                Vec2::ZERO,
                PointerAction::Press(PointerButton::Primary),
            ),
            hit,
        );
        gestures.input(
            &input(PointerId::Mouse, Vec2::ZERO, PointerAction::Cancel),
            hit,
        );
        assert!(gestures
            .input(
                &input(
                    PointerId::Mouse,
                    Vec2::ZERO,
                    PointerAction::Release(PointerButton::Primary)
                ),
                hit
            )
            .is_none());
        for action in [
            PointerAction::Press(PointerButton::Secondary),
            PointerAction::Release(PointerButton::Secondary),
        ] {
            assert!(gestures
                .input(&input(PointerId::Mouse, Vec2::ZERO, action), hit)
                .is_none());
        }
    }

    #[test]
    fn touch_gestures_do_not_cancel_each_other() {
        let mut gestures = Gestures::default();
        let hit = |_: &Location| Some(Entity::PLACEHOLDER);
        for pointer in [PointerId::Touch(1), PointerId::Touch(2)] {
            gestures.input(
                &input(
                    pointer,
                    Vec2::ZERO,
                    PointerAction::Press(PointerButton::Primary),
                ),
                hit,
            );
        }
        gestures.input(
            &input(
                PointerId::Touch(1),
                Vec2::new(0.0, 20.0),
                PointerAction::Move {
                    delta: Vec2::new(0.0, 20.0),
                },
            ),
            hit,
        );
        assert!(gestures
            .input(
                &input(
                    PointerId::Touch(1),
                    Vec2::ZERO,
                    PointerAction::Release(PointerButton::Primary)
                ),
                hit
            )
            .is_none());
        assert!(gestures
            .input(
                &input(
                    PointerId::Touch(2),
                    Vec2::ZERO,
                    PointerAction::Release(PointerButton::Primary)
                ),
                hit
            )
            .is_some());
    }
}
