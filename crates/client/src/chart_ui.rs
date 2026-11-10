//! Tactical chart: public mission geometry, own ship and shared estimates only.
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;

use crate::button_activation::{self, ButtonActivated};
use crate::chart::{self, ChartView};
use crate::mission_ui::MissionUi;
use crate::network::{GameState, NetworkReceiveSet};
use crate::periscope_ui::PeriscopeUi;

const GRID_SLOTS: usize = 32;
const TRACK_SLOTS: usize = 16;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ChartViewSet;

#[derive(Component)]
pub(crate) struct ChartCanvas;
#[derive(Component)]
struct ChartRoot;
#[derive(Component)]
pub(crate) struct ChartHeading;
#[derive(Component)]
struct ExtractionBanner;
#[derive(Component)]
struct ExtractionBannerText;
#[derive(Component)]
struct Footer;
#[derive(Component, Clone, Copy)]
enum Action {
    ZoomIn,
    ZoomOut,
    Recenter,
    Extraction,
}
#[derive(Component, Clone, Copy)]
enum Mark {
    LongitudeLine(usize),
    LatitudeLine(usize),
    Longitude(usize),
    Latitude(usize),
    Submarine,
    OwnLabel,
    Heading,
    ExtractionRing,
    ExtractionLabel,
    Boundary,
    ScaleLine,
    ScaleLabel,
    North,
    Track(usize),
    TrackLabel(usize),
    Uncertainty(usize),
    Bearing(usize),
}

pub struct ChartUiPlugin;
impl Plugin for ChartUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChartView>()
            .add_systems(
                Update,
                update_view.after(NetworkReceiveSet).in_set(ChartViewSet),
            )
            .add_systems(
                Update,
                update_feedback_banner
                    .after(NetworkReceiveSet)
                    .before(ChartViewSet),
            )
            .add_systems(Update, paint.after(ChartViewSet));
    }
}

pub(crate) fn spawn_feedback_banner(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            ExtractionBanner,
            BackgroundColor(Color::srgb(0.98, 0.68, 0.18)),
            BorderColor::all(Color::srgb(0.65, 0.39, 0.08)),
            Node {
                display: Display::None,
                width: percent(100),
                min_height: px(44),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(8), px(6)),
                border: UiRect::all(px(1)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_child((
            ExtractionBannerText,
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(Color::srgb(0.08, 0.05, 0.01)),
            Node {
                width: percent(100),
                min_width: px(0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ));
}

fn update_feedback_banner(
    state: Res<GameState>,
    mut text: Single<&mut Text, With<ExtractionBannerText>>,
    mut nodes: Query<
        (
            &mut Node,
            Has<ExtractionBanner>,
            Has<ChartHeading>,
            Has<ChartRoot>,
            Has<ChartCanvas>,
        ),
        Or<(
            With<ExtractionBanner>,
            With<ChartHeading>,
            With<ChartRoot>,
            With<ChartCanvas>,
        )>,
    >,
) {
    if !state.is_changed() {
        return;
    }
    let notice = state.submarine.as_ref().and_then(|sub| {
        crate::mission_feedback::extraction_notice(state.mission, sub.common.x, sub.common.y)
    });
    text.0 = notice
        .map(|message| message.replace('\n', " / "))
        .unwrap_or_default();
    for (mut node, banner, heading, root, canvas) in &mut nodes {
        if banner {
            node.display = if notice.is_some() {
                Display::Flex
            } else {
                Display::None
            };
        } else if heading {
            // Replace the heading, rather than stacking another header over a short chart.
            node.display = if notice.is_some() {
                Display::None
            } else {
                Display::Flex
            };
        } else if root {
            node.min_height = px(if notice.is_some() { 132 } else { 154 });
        } else if canvas {
            node.min_height = px(if notice.is_some() { 50 } else { 72 });
        }
    }
}

pub(crate) fn spawn(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            ChartRoot,
            Node {
                width: percent(100),
                flex_grow: 1.0,
                min_height: px(154),
                flex_direction: FlexDirection::Column,
                row_gap: px(4),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: percent(100),
                min_height: px(44),
                flex_shrink: 0.0,
                column_gap: px(4),
                ..default()
            })
            .with_children(|bar| {
                for (action, label, width) in [
                    (Action::ZoomOut, "-", 44.0),
                    (Action::ZoomIn, "+", 44.0),
                    (Action::Recenter, "RECENTRER", 76.0),
                    (Action::Extraction, "EXTRACTION", 76.0),
                ] {
                    let mut button = bar.spawn((
                        action,
                        Button,
                        Node {
                            min_width: px(width),
                            min_height: px(44),
                            flex_grow: if width > 44.0 { 1.0 } else { 0.0 },
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.04, 0.19, 0.23)),
                    ));
                    button_activation::enable(&mut button);
                    button.with_child((
                        Text::new(label),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::srgb(0.75, 0.95, 1.0)),
                        Pickable::IGNORE,
                    ));
                }
            });
            root.spawn((
                ChartCanvas,
                RelativeCursorPosition::default(),
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    min_height: px(72),
                    position_type: PositionType::Relative,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.025, 0.10, 0.13)),
            ))
            .observe(
                |mut drag: On<Pointer<Drag>>, scale: Res<UiScale>, mut view: ResMut<ChartView>| {
                    if drag.button == PointerButton::Primary {
                        drag.propagate(false);
                        view.pan(drag.delta / scale.0.max(0.01));
                    }
                },
            )
            .with_children(|map| {
                for mark in (0..GRID_SLOTS)
                    .flat_map(|i| {
                        [
                            Mark::LongitudeLine(i),
                            Mark::LatitudeLine(i),
                            Mark::Longitude(i),
                            Mark::Latitude(i),
                        ]
                    })
                    .chain([
                        Mark::Boundary,
                        Mark::ExtractionRing,
                        Mark::ExtractionLabel,
                        Mark::Heading,
                        Mark::Submarine,
                        Mark::OwnLabel,
                        Mark::ScaleLine,
                        Mark::ScaleLabel,
                        Mark::North,
                    ])
                    .chain((0..TRACK_SLOTS).flat_map(|i| {
                        [
                            Mark::Uncertainty(i),
                            Mark::Bearing(i),
                            Mark::Track(i),
                            Mark::TrackLabel(i),
                        ]
                    }))
                {
                    let mut entity = map.spawn((
                        mark,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(match mark {
                            Mark::Track(_) | Mark::TrackLabel(_) | Mark::Bearing(_) => {
                                Color::srgb(1.0, 0.80, 0.35)
                            }
                            Mark::ExtractionLabel => Color::srgb(0.45, 1.0, 0.65),
                            Mark::OwnLabel => Color::srgb(0.60, 0.92, 1.0),
                            _ => Color::srgb(0.40, 0.67, 0.73),
                        }),
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        UiTransform::default(),
                        BackgroundColor(Color::NONE),
                        BorderColor::all(Color::NONE),
                        ZIndex(match mark {
                            Mark::Submarine | Mark::OwnLabel => 5,
                            Mark::Track(_) | Mark::TrackLabel(_) => 4,
                            Mark::ExtractionLabel | Mark::ScaleLabel | Mark::North => 6,
                            _ => 0,
                        }),
                        Pickable::IGNORE,
                        Visibility::Hidden,
                    ));
                    if matches!(mark, Mark::Submarine) {
                        entity.with_children(|sub| {
                            for (left, top, width, height, color) in [
                                (4.0, 12.0, 6.0, 12.0, Color::srgb(0.02, 0.18, 0.23)),
                                (-6.0, 27.0, 26.0, 3.0, Color::srgb(0.35, 0.80, 0.95)),
                                (6.0, -5.0, 2.0, 8.0, Color::WHITE),
                            ] {
                                sub.spawn((
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: px(left),
                                        top: px(top),
                                        width: px(width),
                                        height: px(height),
                                        ..default()
                                    },
                                    BackgroundColor(color),
                                    Pickable::IGNORE,
                                ));
                            }
                        });
                    }
                }
                crate::torpedo_ui::spawn(map);
            });
            root.spawn((
                Footer,
                Text::new("SECTEUR FICTIF // NORD EN HAUT"),
                TextFont {
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.79, 0.83)),
                Node {
                    min_height: px(26),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
        });
}

fn update_view(
    state: Res<GameState>,
    mission_ui: Res<MissionUi>,
    periscope: Res<PeriscopeUi>,
    mut view: ResMut<ChartView>,
    canvas: Single<
        (&ComputedNode, &RelativeCursorPosition, &InheritedVisibility),
        With<ChartCanvas>,
    >,
    buttons: Query<&Action>,
    mut clicks: MessageReader<ButtonActivated>,
    mut wheel: MessageReader<MouseWheel>,
    mut room: Local<Option<shared::RoomId>>,
) {
    let current_room = state.lobby.as_ref().map(|l| l.room_id.clone());
    if *room != current_room {
        *view = ChartView::default();
        *room = current_room;
    }
    let size = canvas.0.size * canvas.0.inverse_scale_factor;
    if size.is_finite() && size.min_element() > 0.0 && view.size != size {
        view.size = size;
    }
    let own = state
        .submarine
        .as_ref()
        .map(|s| Vec2::new(s.common.x, s.common.y));
    let active = state.game_started && canvas.2.get() && !mission_ui.is_open() && !periscope.open;
    for click in clicks.read() {
        if !active {
            continue;
        }
        if let Ok(action) = buttons.get(click.0) {
            match action {
                Action::ZoomIn => view.zoom(1.0 / 1.5),
                Action::ZoomOut => view.zoom(1.5),
                Action::Recenter => view.following = true,
                Action::Extraction => {
                    if let Some(mission) = state.mission {
                        let zone = mission.briefing.extraction;
                        view.center = Vec2::new(zone.x, zone.y);
                        view.half_range = 2000.0;
                        view.following = false;
                    }
                }
            }
        }
    }
    for event in wheel.read() {
        if active && canvas.1.cursor_over() {
            let amount = match event.unit {
                MouseScrollUnit::Line => event.y,
                MouseScrollUnit::Pixel => event.y / 100.0,
            };
            view.zoom(1.2_f32.powf(-amount.clamp(-5.0, 5.0)));
        }
    }
    if view.following {
        if let Some(own) = own {
            if view.center != own {
                view.center = own;
            }
        }
    }
}

fn set_line(node: &mut Node, transform: &mut UiTransform, a: Vec2, b: Vec2, thickness: f32) {
    let delta = b - a;
    let center = (a + b) * 0.5;
    node.left = px(center.x - delta.length() * 0.5);
    node.top = px(center.y - thickness * 0.5);
    node.width = px(delta.length());
    node.height = px(thickness);
    transform.rotation = Rot2::radians(delta.y.atan2(delta.x));
}

fn set_circle(node: &mut Node, center: Vec2, radius: f32) {
    node.left = px(center.x - radius);
    node.top = px(center.y - radius);
    node.width = px(2.0 * radius);
    node.height = px(2.0 * radius);
    node.border = UiRect::all(px(1));
    node.border_radius = BorderRadius::all(percent(50));
}

fn paint(
    state: Res<GameState>,
    view: Res<ChartView>,
    mut marks: Query<(
        &Mark,
        &mut Text,
        &mut Node,
        &mut UiTransform,
        &mut BackgroundColor,
        &mut BorderColor,
        &mut Visibility,
    )>,
    mut footer: Single<&mut Text, (With<Footer>, Without<Mark>)>,
) {
    if !state.is_changed() && !view.is_changed() {
        return;
    }
    let Some(sub) = state.submarine.as_ref() else {
        return;
    };
    let own = Vec2::new(sub.common.x, sub.common.y);
    let extraction_waiting =
        crate::mission_feedback::extraction_notice(state.mission, own.x, own.y).is_some();
    let scale = view.scale();
    let bottom_left = view.unproject(Vec2::new(0.0, view.size.y));
    let top_right = view.unproject(Vec2::new(view.size.x, 0.0));
    let (lat, lon) = chart::coordinates(own);
    footer.0 = format!(
        "{} / {} // CAP {:03.0} deg\nSECTEUR FICTIF / {}",
        chart::coordinate_label(lat, true),
        chart::coordinate_label(lon, false),
        sub.common.heading,
        if view.following { "SUIVI" } else { "VUE LIBRE" }
    );
    for (mark, mut text, mut node, mut transform, mut background, mut border, mut visibility) in
        &mut marks
    {
        *visibility = Visibility::Hidden;
        // Reset pooled shape attributes before reuse as the viewport changes.
        node.border = UiRect::ZERO;
        node.border_radius = BorderRadius::ZERO;
        node.width = Val::Auto;
        node.height = Val::Auto;
        transform.rotation = Rot2::IDENTITY;
        background.0 = Color::NONE;
        *border = BorderColor::all(Color::NONE);
        text.0.clear();
        let mut position = Vec2::ZERO;
        match *mark {
            Mark::LongitudeLine(i)
            | Mark::Longitude(i)
            | Mark::LatitudeLine(i)
            | Mark::Latitude(i) => {
                let longitude = matches!(mark, Mark::LongitudeLine(_) | Mark::Longitude(_));
                let step = chart::grid_step(scale, longitude);
                let lower = if longitude {
                    bottom_left.x
                } else {
                    bottom_left.y
                };
                let value = (lower / step).ceil() * step + i as f32 * step;
                let point = view.project(if longitude {
                    Vec2::new(value, view.center.y)
                } else {
                    Vec2::new(view.center.x, value)
                });
                if (longitude && (point.x < 0.0 || point.x > view.size.x))
                    || (!longitude && (point.y < 0.0 || point.y > view.size.y))
                {
                    continue;
                }
                if matches!(mark, Mark::LongitudeLine(_) | Mark::LatitudeLine(_)) {
                    background.0 = Color::srgba(0.20, 0.46, 0.51, 0.35);
                    node.left = px(if longitude { point.x } else { 0.0 });
                    node.top = px(if longitude { 0.0 } else { point.y });
                    node.width = px(if longitude { 1.0 } else { view.size.x });
                    node.height = px(if longitude { view.size.y } else { 1.0 });
                } else {
                    // Keep the lower-left scale and the top longitude graduations legible.
                    if !longitude && (point.y < 18.0 || point.y > view.size.y - 48.0) {
                        continue;
                    }
                    let (latitude, lon) = chart::coordinates(if longitude {
                        Vec2::new(value, 0.0)
                    } else {
                        Vec2::new(0.0, value)
                    });
                    text.0 =
                        chart::coordinate_label(if longitude { lon } else { latitude }, !longitude);
                    position = if longitude {
                        Vec2::new(point.x.max(4.0).min((view.size.x - 90.0).max(4.0)), 3.0)
                    } else {
                        Vec2::new(4.0, (point.y + 3.0).min((view.size.y - 16.0).max(0.0)))
                    };
                }
            }
            Mark::Boundary => {
                let Some(mission) = state.mission else {
                    continue;
                };
                let extent = mission.briefing.operation_half_extent;
                let start = view.project(Vec2::new(-extent, extent));
                node.left = px(start.x);
                node.top = px(start.y);
                node.width = px(extent * 2.0 * scale);
                node.height = px(extent * 2.0 * scale);
                node.border = UiRect::all(px(1));
                *border = BorderColor::all(Color::srgba(0.35, 0.62, 0.70, 0.5));
            }
            Mark::Submarine | Mark::OwnLabel | Mark::Heading => {
                let point = view.project(own);
                if matches!(mark, Mark::Heading) {
                    let end = point
                        + Vec2::new(
                            sub.common.heading.to_radians().sin(),
                            -sub.common.heading.to_radians().cos(),
                        ) * 65.0;
                    let Some((a, b)) = view.clip_line(point, end) else {
                        continue;
                    };
                    set_line(&mut node, &mut transform, a, b, 1.0);
                    background.0 = Color::srgba(0.50, 0.85, 1.0, 0.7);
                } else {
                    if !view.contains(point) {
                        continue;
                    }
                    if matches!(mark, Mark::Submarine) {
                        node.left = px(point.x - 7.0);
                        node.top = px(point.y - 19.0);
                        node.width = px(14);
                        node.height = px(38);
                        node.border_radius = BorderRadius::all(percent(50));
                        background.0 = Color::srgb(0.35, 0.80, 0.95);
                        transform.rotation = Rot2::degrees(sub.common.heading);
                    } else {
                        text.0 = "SOUS-MARIN".into();
                        position = point + Vec2::new(16.0, 18.0);
                    }
                }
            }
            Mark::ExtractionRing | Mark::ExtractionLabel => {
                let Some(mission) = state.mission else {
                    continue;
                };
                let zone = mission.briefing.extraction;
                let center = view.project(Vec2::new(zone.x, zone.y));
                let radius = zone.radius * scale;
                if matches!(mark, Mark::ExtractionRing) {
                    set_circle(&mut node, center, radius);
                    node.border = UiRect::all(px(2));
                    *border = BorderColor::all(if extraction_waiting {
                        Color::srgb(1.0, 0.75, 0.25)
                    } else {
                        Color::srgb(0.4, 0.95, 0.6)
                    });
                    background.0 = if extraction_waiting {
                        Color::srgba(1.0, 0.65, 0.15, 0.12)
                    } else {
                        Color::srgba(0.20, 0.75, 0.40, 0.12)
                    };
                } else {
                    if !view.contains(center) {
                        continue;
                    }
                    text.0 = format!(
                        "EXTRACTION\nR {:.0} m / {:.0} m",
                        zone.radius,
                        own.distance(Vec2::new(zone.x, zone.y))
                    );
                    position = Vec2::new(
                        (center.x + radius + 6.0).min((view.size.x - 120.0).max(4.0)),
                        (center.y + 5.0).min((view.size.y - 28.0).max(0.0)),
                    );
                }
            }
            Mark::ScaleLine | Mark::ScaleLabel => {
                let meters = [0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0]
                    .into_iter()
                    .map(|n| n * chart::METERS_PER_NM)
                    .filter(|m| m * scale <= (view.size.x * 0.3).min(140.0))
                    .last()
                    .unwrap_or(0.1 * chart::METERS_PER_NM);
                if matches!(mark, Mark::ScaleLine) {
                    set_line(
                        &mut node,
                        &mut transform,
                        Vec2::new(12.0, view.size.y - 9.0),
                        Vec2::new(12.0 + meters * scale, view.size.y - 9.0),
                        2.0,
                    );
                    background.0 = Color::srgb(0.70, 0.85, 0.85);
                } else {
                    text.0 = format!("{:.2} MN / {:.0} m", meters / chart::METERS_PER_NM, meters);
                    position = Vec2::new(12.0, view.size.y - 25.0);
                }
            }
            Mark::North => {
                text.0 = "^ N".into();
                position = Vec2::new(view.size.x - 32.0, 20.0);
            }
            Mark::Track(i) | Mark::TrackLabel(i) | Mark::Uncertainty(i) | Mark::Bearing(i) => {
                let Some(track) = state.tactical.as_ref().and_then(|t| t.shared_tracks.get(i))
                else {
                    continue;
                };
                let dir = chart::direction(track.bearing);
                if let Some(distance) = track.distance {
                    if matches!(mark, Mark::Bearing(_)) {
                        continue;
                    }
                    let point = view.project(own + dir * distance);
                    match mark {
                        Mark::Uncertainty(_) => {
                            let radius = track.distance_uncertainty.unwrap_or(0.0).hypot(
                                distance * track.bearing_uncertainty.to_radians().sin().abs(),
                            ) * scale;
                            set_circle(&mut node, point, radius.max(3.0));
                            *border = BorderColor::all(Color::srgba(0.95, 0.7, 0.25, 0.35));
                        }
                        Mark::Track(_) => {
                            if !view.contains(point) {
                                continue;
                            }
                            node.left = px(point.x - 4.0);
                            node.top = px(point.y - 4.0);
                            node.width = px(8);
                            node.height = px(8);
                            transform.rotation = Rot2::degrees(45.0);
                            background.0 = Color::srgb(1.0, 0.78, 0.25);
                        }
                        _ => {
                            if !view.contains(point) {
                                continue;
                            }
                            text.0 = format!("P{} EST.", track.id.0);
                            position = point + Vec2::new(7.0, -15.0);
                        }
                    }
                } else {
                    if !matches!(mark, Mark::Bearing(_) | Mark::TrackLabel(_)) {
                        continue;
                    }
                    let length =
                        own.distance(view.center) + (top_right - bottom_left).length() * 2.0;
                    let Some((a, b)) =
                        view.clip_line(view.project(own), view.project(own + dir * length))
                    else {
                        continue;
                    };
                    if matches!(mark, Mark::Bearing(_)) {
                        set_line(&mut node, &mut transform, a, b, 1.0);
                        background.0 = Color::srgba(1.0, 0.78, 0.25, 0.3);
                    } else {
                        text.0 = format!("P{} / DIST. ?", track.id.0);
                        position = Vec2::new(
                            b.x.clamp(4.0, (view.size.x - 110.0).max(4.0)),
                            b.y.clamp(18.0, (view.size.y - 45.0).max(18.0)),
                        );
                    }
                }
            }
        }
        if !text.0.is_empty() {
            node.left = px(position.x);
            node.top = px(position.y);
        }
        *visibility = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn extraction_feedback_has_a_dedicated_collapsible_banner() {
        let mut world = World::new();
        world
            .run_system_once(|mut commands: Commands| {
                commands
                    .spawn(Node::default())
                    .with_children(spawn_feedback_banner);
            })
            .unwrap();
        let banner = world
            .query_filtered::<Entity, With<ExtractionBanner>>()
            .single(&world)
            .unwrap();
        assert_eq!(world.get::<Node>(banner).unwrap().display, Display::None);
        let message = world
            .query_filtered::<Entity, With<ExtractionBannerText>>()
            .single(&world)
            .unwrap();
        assert_eq!(world.get::<ChildOf>(message).unwrap().parent(), banner);
        assert_eq!(
            world.get::<TextFont>(message).unwrap().font_size,
            FontSize::Px(16.0)
        );
        assert_eq!(world.get::<Node>(banner).unwrap().flex_shrink, 0.0);

        let heading = world.spawn((ChartHeading, Node::default())).id();
        let root = world.spawn((ChartRoot, Node::default())).id();
        let canvas = world.spawn((ChartCanvas, Node::default())).id();
        let mut state = GameState::default();
        state.mission = Some(shared::MissionProgress {
            briefing: shared::MissionBriefing::level_one(0),
            phase: shared::MissionPhase::Attack,
            elapsed_seconds: 0.0,
        });
        state.submarine = Some(shared::SubmarineSnapshot {
            common: shared::CommonMeasurements {
                x: 0.0,
                y: -2000.0,
                heading: 0.0,
                speed: 0.0,
                depth: 0.0,
                dive_state: shared::DiveState::Surface,
                acoustic_level: shared::AcousticLevel::Low,
                alerts: vec![],
                damage: shared::DamageSummary::default(),
            },
            pilot: None,
            engineering: None,
        });
        world.insert_resource(state);
        world.run_system_once(update_feedback_banner).unwrap();
        assert_eq!(world.get::<Node>(banner).unwrap().display, Display::Flex);
        assert!(world
            .get::<Text>(message)
            .unwrap()
            .0
            .contains("MISSION TOUJOURS EN COURS"));
        assert_eq!(world.get::<Node>(heading).unwrap().display, Display::None);
        assert_eq!(world.get::<Node>(root).unwrap().min_height, px(132));
        assert_eq!(world.get::<Node>(canvas).unwrap().min_height, px(50));

        world
            .resource_mut::<GameState>()
            .submarine
            .as_mut()
            .unwrap()
            .common
            .y = 0.0;
        world.run_system_once(update_feedback_banner).unwrap();
        assert_eq!(world.get::<Node>(banner).unwrap().display, Display::None);
        assert!(world.get::<Text>(message).unwrap().0.is_empty());
        assert_eq!(world.get::<Node>(heading).unwrap().display, Display::Flex);
        assert_eq!(world.get::<Node>(root).unwrap().min_height, px(154));

        {
            let mut state = world.resource_mut::<GameState>();
            state.submarine.as_mut().unwrap().common.y = -2000.0;
            state.mission.as_mut().unwrap().phase = shared::MissionPhase::Victory;
        }
        world.run_system_once(update_feedback_banner).unwrap();
        assert_eq!(world.get::<Node>(banner).unwrap().display, Display::None);
    }
}
