//! Presentation-only journals over the tactical chart; no gameplay state lives here.
use crate::button_activation::{self, ButtonActivated};
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    ui::RelativeCursorPosition,
    window::PrimaryWindow,
};

#[derive(Component, Clone, Copy)]
struct Toggle(usize);
#[derive(Component)]
pub(crate) struct Journal(usize);
#[derive(Component)]
struct ScrollStart(Vec2);
#[derive(Resource, Default)]
struct Layout {
    open: [bool; 2],
    desktop: Option<bool>,
}

impl Layout {
    fn resize(&mut self, wide: bool) {
        if self.desktop != Some(wide) {
            self.desktop = Some(wide);
            self.open = [wide; 2];
        }
    }

    fn toggle(&mut self, index: usize) {
        let open = !self.open[index];
        if self.desktop != Some(true) {
            self.open = [false; 2];
        }
        self.open[index] = open;
    }
}

pub struct TacticalLayoutPlugin;
impl Plugin for TacticalLayoutPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Layout>().add_systems(Update, update);
    }
}

pub(crate) fn spawn(
    parent: &mut ChildSpawnerCommands,
    sonar: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn(Node {
            width: percent(100),
            flex_grow: 1.0,
            min_height: px(0),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|area| {
            area.spawn(Node {
                width: percent(100),
                flex_shrink: 0.0,
                column_gap: px(4),
                ..default()
            })
            .with_children(|bar| {
                for (index, label) in ["SONAR / JOURNAL", "VISUEL / ARMEMENT"]
                    .into_iter()
                    .enumerate()
                {
                    toggle(bar, index, label);
                }
            });
            area.spawn(Node {
                width: percent(100),
                min_height: px(0),
                flex_grow: 1.0,
                ..default()
            })
            .with_children(|map| {
                crate::chart_ui::spawn(map);
            });
            journal(area, 0, sonar);
            journal(area, 1, |content| {
                crate::periscope_ui::spawn_shared_contacts(content);
                content.spawn((
                    crate::weapons_ui::WeaponStatus,
                    Text::new("ARMEMENT"),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Node {
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            });
        });
    crate::weapons_ui::spawn(parent);
}

fn toggle(parent: &mut ChildSpawnerCommands, index: usize, label: &str) {
    let mut button = parent.spawn((
        Toggle(index),
        Button,
        Node {
            min_height: px(44),
            min_width: px(0),
            flex_grow: 1.0,
            flex_basis: px(0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.04, 0.18, 0.23)),
    ));
    button_activation::enable(&mut button);
    button.with_child((
        Text::new(label),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Pickable::IGNORE,
    ));
}

fn journal(
    parent: &mut ChildSpawnerCommands,
    index: usize,
    content: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent.spawn((Journal(index), RelativeCursorPosition::default(), Node { display: Display::None, position_type: PositionType::Absolute,
        top: px(48), bottom: px(26), width: px(280), max_width: percent(90),
        flex_direction: FlexDirection::Column, padding: UiRect::all(px(6)), ..default() },
        ZIndex(20), BackgroundColor(Color::srgba(0.01, 0.055, 0.075, 0.86))))
        .with_children(|panel| {
            toggle(panel, index, "FERMER / REPLIER");
            panel.spawn((ScrollStart(Vec2::ZERO), ScrollPosition::default(), RelativeCursorPosition::default(),
                Node { width: percent(100), min_height: px(0), flex_grow: 1.0,
                    flex_direction: FlexDirection::Column, row_gap: px(8), overflow: Overflow::scroll_y(), ..default() }))
                .observe(|mut drag: On<Pointer<Drag>>, scale: Res<UiScale>, mut lists: Query<(&ComputedNode, &ScrollStart, &mut ScrollPosition)>| {
                    if let Ok((node, start, mut position)) = lists.get_mut(drag.event_target()) {
                        let range = (node.content_size.y - node.size.y).max(0.0) * node.inverse_scale_factor;
                        position.0.y = (start.0.y - drag.distance.y / scale.0).clamp(0.0, range);
                    }
                    drag.propagate(false);
                })
                .observe(|mut drag: On<Pointer<DragStart>>, mut lists: Query<(&ScrollPosition, &mut ScrollStart)>| {
                    if let Ok((position, mut start)) = lists.get_mut(drag.event_target()) { start.0 = position.0; }
                    drag.propagate(false);
                })
                .with_children(content);
        });
}

fn desktop(width: f32, height: f32) -> bool {
    width >= 960.0 && height > 500.0
}

fn update(
    window: Single<&Window, With<PrimaryWindow>>,
    mut layout: ResMut<Layout>,
    toggles: Query<&Toggle>,
    mut clicks: MessageReader<ButtonActivated>,
    mut panels: Query<(&Journal, &mut Node)>,
    mut lists: Query<
        (
            &ComputedNode,
            &RelativeCursorPosition,
            &InheritedVisibility,
            &mut ScrollPosition,
        ),
        With<ScrollStart>,
    >,
    mut wheel: MessageReader<MouseWheel>,
) {
    let wide = desktop(window.width(), window.height());
    layout.resize(wide);
    for click in clicks.read() {
        if let Ok(toggle) = toggles.get(click.0) {
            layout.toggle(toggle.0);
        }
    }
    for (journal, mut node) in &mut panels {
        node.display = if layout.open[journal.0] {
            Display::Flex
        } else {
            Display::None
        };
        node.width = if wide { px(280) } else { percent(90) };
        node.top = px(if window.height() <= 500.0 { 0 } else { 92 });
        node.bottom = px(if window.height() <= 500.0 { 0 } else { 26 });
        node.left = if journal.0 == 0 { px(0) } else { Val::Auto };
        node.right = if journal.0 == 1 { px(0) } else { Val::Auto };
    }
    for event in wheel.read() {
        for (node, cursor, visible, mut position) in &mut lists {
            if visible.get() && cursor.cursor_over() {
                let delta = match event.unit {
                    MouseScrollUnit::Line => event.y * 44.0,
                    MouseScrollUnit::Pixel => event.y,
                };
                let range =
                    (node.content_size.y - node.size.y).max(0.0) * node.inverse_scale_factor;
                position.0.y = (position.0.y - delta).clamp(0.0, range);
            }
        }
    }
}

pub(crate) fn pointer_over_journal(
    panels: &Query<(&RelativeCursorPosition, &Node, &InheritedVisibility), With<Journal>>,
) -> bool {
    panels.iter().any(|(cursor, node, visible)| {
        node.display != Display::None && visible.get() && cursor.cursor_over()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phones_including_landscape_use_drawers() {
        assert!(!desktop(390.0, 844.0));
        assert!(!desktop(1000.0, 450.0));
        assert!(!desktop(959.0, 800.0));
        assert!(desktop(960.0, 800.0));
    }

    #[test]
    fn mobile_drawers_are_exclusive_and_close_on_second_tap() {
        let mut layout = Layout::default();
        layout.resize(false);
        assert_eq!(layout.open, [false, false]);
        layout.toggle(0);
        assert_eq!(layout.open, [true, false]);
        layout.toggle(1);
        assert_eq!(layout.open, [false, true]);
        layout.toggle(1);
        assert_eq!(layout.open, [false, false]);
    }

    #[test]
    fn desktop_panels_collapse_independently_and_resize_preserves_toggles() {
        let mut layout = Layout::default();
        layout.resize(true);
        assert_eq!(layout.open, [true, true]);
        layout.toggle(0);
        layout.resize(true);
        assert_eq!(layout.open, [false, true]);
        layout.resize(false);
        assert_eq!(layout.open, [false, false]);
    }

    #[test]
    fn only_journals_scroll_and_weapon_controls_are_outside_them() {
        let mut app = App::new();
        app.add_systems(Startup, |mut commands: Commands| {
            commands
                .spawn(Node::default())
                .with_children(|parent| spawn(parent, |_| {}));
        });
        app.update();
        let world = app.world_mut();
        let mut scrolls = world.query_filtered::<Entity, With<ScrollPosition>>();
        assert_eq!(scrolls.iter(world).count(), 2);
        let mut statuses = world.query_filtered::<Entity, With<crate::weapons_ui::WeaponStatus>>();
        assert_eq!(statuses.iter(world).count(), 1);
        let mut panels = world.query_filtered::<Entity, With<Journal>>();
        let panels: Vec<_> = panels.iter(world).collect();
        let mut buttons = world.query_filtered::<Entity, (With<Button>, Without<Toggle>)>();
        let buttons: Vec<_> = buttons.iter(world).collect();
        assert_eq!(buttons.len(), 13); // Nine weapon actions and four chart controls.
        for mut entity in buttons {
            while let Some(parent) = world.get::<ChildOf>(entity) {
                entity = parent.parent();
                assert!(!panels.contains(&entity));
                assert!(world.get::<ScrollPosition>(entity).is_none());
            }
        }
    }
}
