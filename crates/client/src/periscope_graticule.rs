//! Visual angular instruments only: uses the same horizontal projection as silhouettes.
use bevy::prelude::*;

use crate::{network::GameState, periscope_ui::View};

const INK: Color = Color::srgba(0.65, 0.95, 0.77, 0.9);

#[derive(Component)]
struct Overlay;

#[derive(Clone, Copy)]
enum Scale {
    Absolute,
    Relative,
}

#[derive(Component)]
struct Mark {
    scale: Scale,
    degree: u16,
    label: bool,
}

#[derive(Component)]
struct Readout(Scale);

#[derive(Component)]
struct ReticleTick(f32);

pub(crate) struct PeriscopeGraticulePlugin;

impl Plugin for PeriscopeGraticulePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update.after(crate::network::NetworkReceiveSet));
    }
}

pub(crate) fn spawn(view: &mut ChildSpawnerCommands) {
    for scale in [Scale::Absolute, Scale::Relative] {
        let upper = matches!(scale, Scale::Absolute);
        view.spawn((
            Overlay,
            Visibility::Hidden,
            Pickable::IGNORE,
            ZIndex(2),
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: if upper { px(0) } else { Val::Auto },
                bottom: if upper { Val::Auto } else { px(0) },
                height: px(36),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.035, 0.025, 0.78)),
        ))
        .with_children(|ribbon| {
            for degree in 0..360 {
                let major = degree % 5 == 0;
                ribbon.spawn((
                    Overlay,
                    Mark {
                        scale,
                        degree,
                        label: false,
                    },
                    Visibility::Hidden,
                    Pickable::IGNORE,
                    Node {
                        position_type: PositionType::Absolute,
                        top: if upper { Val::Auto } else { px(0) },
                        bottom: if upper { px(0) } else { Val::Auto },
                        width: px(1),
                        height: px(if major { 12 } else { 6 }),
                        ..default()
                    },
                    BackgroundColor(INK),
                ));
                if degree % 10 == 0 {
                    ribbon.spawn((
                        Overlay,
                        Mark {
                            scale,
                            degree,
                            label: true,
                        },
                        Visibility::Hidden,
                        Pickable::IGNORE,
                        Text::new(match scale {
                            Scale::Absolute => absolute_label(degree as f32),
                            Scale::Relative => relative_label(degree as f32),
                        }),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(INK),
                        TextLayout::justify(Justify::Center),
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(52),
                            top: if upper { px(2) } else { Val::Auto },
                            bottom: if upper { Val::Auto } else { px(2) },
                            ..default()
                        },
                    ));
                }
            }
            // Fixed cursor: the tapes move underneath it, never the reverse.
            ribbon.spawn((
                Pickable::IGNORE,
                ZIndex(1),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(50),
                    width: px(2),
                    height: px(16),
                    top: if upper { Val::Auto } else { px(0) },
                    bottom: if upper { px(0) } else { Val::Auto },
                    ..default()
                },
                BackgroundColor(Color::srgb(0.9, 1.0, 0.75)),
            ));
            ribbon.spawn((
                Overlay,
                Readout(scale),
                Visibility::Hidden,
                Pickable::IGNORE,
                ZIndex(1),
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(INK),
                TextLayout::justify(Justify::Center),
                BackgroundColor(Color::srgba(0.0, 0.035, 0.025, 0.9)),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(112),
                    top: if upper { px(38) } else { Val::Auto },
                    bottom: if upper { Val::Auto } else { px(38) },
                    ..default()
                },
            ));
        });
    }
    for offset in [-10.0_f32, -5.0, -2.0, -1.0, 1.0, 2.0, 5.0, 10.0] {
        let height = if offset.abs() >= 5.0 { 12.0 } else { 6.0 };
        view.spawn((
            Overlay,
            ReticleTick(offset),
            Visibility::Hidden,
            Pickable::IGNORE,
            ZIndex(1),
            Node {
                position_type: PositionType::Absolute,
                top: percent(60),
                margin: UiRect::top(px(-height * 0.5)),
                width: px(1),
                height: px(height),
                ..default()
            },
            BackgroundColor(INK),
        ));
    }
}

fn update(
    state: Res<GameState>,
    view: Single<Ref<ComputedNode>, With<View>>,
    mut overlays: Query<
        (
            &mut Node,
            &mut Visibility,
            Option<&Mark>,
            Option<&Readout>,
            Option<&ReticleTick>,
            Option<&mut Text>,
        ),
        With<Overlay>,
    >,
) {
    if !state.is_changed() && !view.is_changed() {
        return;
    }
    let scope = state.periscope.as_ref().filter(|scope| scope.raised);
    let heading = state.submarine.as_ref().map_or(0.0, |s| s.common.heading);
    let width = view.size.x * view.inverse_scale_factor;
    for (mut node, mut visibility, mark, readout, reticle, text) in &mut overlays {
        let Some(scope) = scope else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        visibility.set_if_neq(Visibility::Inherited);
        if let Some(mark) = mark {
            let bearing = mark.degree as f32
                + match mark.scale {
                    Scale::Absolute => 0.0,
                    Scale::Relative => heading,
                };
            let offset = angular_delta(bearing, scope.bearing);
            if offset.abs() > scope.field_of_view * 0.5 {
                visibility.set_if_neq(Visibility::Hidden);
                continue;
            }
            let center = horizontal_position(offset, width, scope.field_of_view);
            if mark.label && (center < 26.0 || center > width - 26.0) {
                visibility.set_if_neq(Visibility::Hidden);
                continue;
            }
            let x = center - if mark.label { 26.0 } else { 0.5 };
            if node.left != px(x) {
                node.left = px(x);
            }
        }
        if let Some(readout) = readout {
            let x = width * 0.5 - 56.0;
            if node.left != px(x) {
                node.left = px(x);
            }
            if let Some(mut text) = text {
                let value = match readout.0 {
                    Scale::Absolute => format!("ABS {} deg", absolute_label(scope.bearing)),
                    Scale::Relative => format!(
                        "REL {}",
                        relative_label(angular_delta(scope.bearing, heading))
                    ),
                };
                if text.0 != value {
                    text.0 = value;
                }
            }
        }
        if let Some(tick) = reticle {
            if tick.0.abs() > scope.field_of_view * 0.5 {
                visibility.set_if_neq(Visibility::Hidden);
                continue;
            }
            let x = horizontal_position(tick.0, width, scope.field_of_view) - 0.5;
            if node.left != px(x) {
                node.left = px(x);
            }
        }
    }
}

fn angular_delta(bearing: f32, origin: f32) -> f32 {
    (bearing - origin + 180.0).rem_euclid(360.0) - 180.0
}

fn horizontal_position(offset: f32, width: f32, field_of_view: f32) -> f32 {
    width * (0.5 + offset / field_of_view)
}

fn absolute_label(bearing: f32) -> String {
    format!("{:03}", (bearing.rem_euclid(360.0).round() as u16) % 360)
}

fn relative_label(angle: f32) -> String {
    let angle = angular_delta(angle, 0.0).round() as i32;
    match angle {
        0 => "AV 0".into(),
        -180 | 180 => "AR 180".into(),
        angle if angle > 0 => format!("TD {angle}"),
        angle => format!("BD {}", -angle),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compass_wrap_is_continuous_and_never_labels_360() {
        assert_eq!(absolute_label(359.9), "000");
        assert_eq!(absolute_label(-1.0), "359");
        assert_eq!(absolute_label(20.0), "020");
        assert_eq!(angular_delta(0.0, 359.0), 1.0);
        assert_eq!(angular_delta(359.0, 0.0), -1.0);
        assert_eq!(horizontal_position(1.0, 320.0, 40.0), 168.0);
        assert_eq!(horizontal_position(-20.0, 320.0, 40.0), 0.0);
        assert_eq!(horizontal_position(20.0, 320.0, 40.0), 320.0);
    }

    #[test]
    fn relative_tape_depends_on_ship_heading_not_north() {
        assert_eq!(relative_label(0.0), "AV 0");
        assert_eq!(relative_label(20.0), "TD 20");
        assert_eq!(relative_label(340.0), "BD 20");
        assert_eq!(relative_label(180.0), "AR 180");
        assert_eq!(relative_label(-180.0), "AR 180");
        assert_eq!(relative_label(angular_delta(20.0, 40.0)), "BD 20");
        assert_eq!(angular_delta(40.0 + 10.0, 20.0), 30.0);
    }
}
