//! Shared non-modal alerts: persistent loss takes priority over audible explosions.
use bevy::prelude::*;

use crate::network::{GameState, NetworkReceiveSet};

const ALERT_SECONDS: f32 = 3.0;

#[derive(Resource, Default)]
struct ExplosionAlert {
    seen: u64,
    count: u64,
    remaining: f32,
}

impl ExplosionAlert {
    fn advance(&mut self, received: u64, dt: f32, running: bool) {
        if !running || received < self.seen {
            self.seen = received;
            self.count = 0;
            self.remaining = 0.0;
            return;
        }
        self.remaining = (self.remaining - dt).max(0.0);
        if self.remaining == 0.0 {
            self.count = 0;
        }
        let incoming = received - self.seen;
        self.seen = received;
        if incoming > 0 {
            self.count = self.count.saturating_add(incoming);
            self.remaining = ALERT_SECONDS;
        }
    }

    fn label(&self) -> String {
        if self.count <= 1 {
            "EXPLOSION ENTENDUE".into()
        } else {
            format!("EXPLOSION ENTENDUE x {}", self.count)
        }
    }
}

#[derive(Component)]
struct ExplosionBanner;

pub struct ExplosionUiPlugin;

impl Plugin for ExplosionUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExplosionAlert>()
            .add_systems(Startup, spawn_banner)
            .add_systems(Update, update_banner.after(NetworkReceiveSet));
    }
}

fn spawn_banner(mut commands: Commands) {
    commands.spawn((
        ExplosionBanner,
        Text::new("EXPLOSION ENTENDUE"),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.96, 0.8)),
        TextLayout::justify(Justify::Center),
        // The top strip only covers station status, never the action controls.
        Node {
            position_type: PositionType::Absolute,
            top: px(8),
            left: px(8),
            right: px(8),
            height: px(32),
            padding: UiRect::axes(px(8), px(6)),
            border: UiRect::all(px(1)),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(Color::srgb(0.48, 0.13, 0.015)),
        BorderColor::all(Color::srgb(1.0, 0.65, 0.15)),
        GlobalZIndex(100),
        Pickable::IGNORE,
        Visibility::Hidden,
    ));
}

fn update_banner(
    state: Res<GameState>,
    time: Res<Time<Real>>,
    mut alert: ResMut<ExplosionAlert>,
    mut banner: Single<
        (
            &mut Text,
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<ExplosionBanner>,
    >,
) {
    alert.advance(
        state.audible_explosions,
        time.delta_secs(),
        state.game_started,
    );
    // A full snapshot is authoritative, even when the destruction event was missed
    // or the connection closed after the loss.
    let lost = state
        .submarine
        .as_ref()
        .is_some_and(|submarine| submarine.common.damage.destroyed);
    banner.1.set_if_neq(if lost || alert.remaining > 0.0 {
        Visibility::Visible
    } else {
        Visibility::Hidden
    });
    if lost {
        banner.0 .0 = "SOUS-MARIN PERDU".into();
        banner.2 .0 = Color::srgb(0.55, 0.025, 0.035);
        *banner.3 = BorderColor::all(Color::srgb(1.0, 0.3, 0.3));
    } else {
        banner.0 .0 = alert.label();
        banner.2 .0 = Color::srgb(0.48, 0.13, 0.015);
        *banner.3 = BorderColor::all(Color::srgb(1.0, 0.65, 0.15));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_expires_in_three_seconds_without_replaying_history() {
        let mut alert = ExplosionAlert::default();
        alert.advance(1, 0.0, true);
        assert_eq!(alert.label(), "EXPLOSION ENTENDUE");
        alert.advance(1, 2.5, true);
        assert!(alert.remaining > 0.0);
        alert.advance(1, 0.5, true);
        assert_eq!(alert.remaining, 0.0);
        alert.advance(1, 0.05, true);
        assert_eq!(alert.remaining, 0.0);
    }

    #[test]
    fn simultaneous_and_subsequent_explosions_are_counted_and_restart_the_timer() {
        let mut alert = ExplosionAlert::default();
        alert.advance(3, 0.05, true);
        assert_eq!(alert.label(), "EXPLOSION ENTENDUE x 3");
        alert.advance(5, 2.9, true);
        assert_eq!(alert.count, 5);
        assert_eq!(alert.remaining, ALERT_SECONDS);
        alert.advance(5, 3.0, true);
        alert.advance(6, 0.05, true);
        assert_eq!(alert.count, 1);
    }

    #[test]
    fn bursts_larger_than_the_history_and_lobby_resets_are_handled() {
        let mut alert = ExplosionAlert::default();
        alert.advance(130, 0.05, true);
        assert_eq!(alert.count, 130);
        alert.advance(130, 0.05, false);
        assert_eq!(alert.remaining, 0.0);
        alert.advance(130, 0.05, true);
        assert_eq!(alert.remaining, 0.0);
        alert.advance(0, 0.05, true);
        alert.advance(1, 0.05, true);
        assert_eq!(alert.count, 1);
    }

    #[test]
    fn loss_from_snapshot_is_persistent_and_overrides_explosions_without_an_event() {
        let mut app = App::new();
        app.init_resource::<GameState>()
            .init_resource::<Time<Real>>()
            .init_resource::<ExplosionAlert>()
            .add_systems(Startup, spawn_banner)
            .add_systems(Update, update_banner);
        app.world_mut().resource_mut::<GameState>().submarine = Some(shared::SubmarineSnapshot {
            common: shared::CommonMeasurements {
                x: 0.0,
                y: 0.0,
                heading: 0.0,
                speed: 0.0,
                depth: 100.0,
                dive_state: shared::DiveState::Submerged,
                acoustic_level: shared::AcousticLevel::Silent,
                alerts: vec![],
                damage: shared::DamageSummary {
                    destroyed: true,
                    ..default()
                },
            },
            pilot: None,
            engineering: None,
        });
        // Also works after disconnect: game_started defaults to false.
        app.update();
        for _ in 0..2 {
            let world = app.world_mut();
            let mut query = world
                .query_filtered::<(&Text, &Visibility, &BackgroundColor), With<ExplosionBanner>>();
            let (text, visibility, background) = query.single(world).unwrap();
            assert_eq!(text.0, "SOUS-MARIN PERDU");
            assert_eq!(*visibility, Visibility::Visible);
            assert_eq!(background.0, Color::srgb(0.55, 0.025, 0.035));
            let mut state = world.resource_mut::<GameState>();
            state.game_started = true;
            state.audible_explosions += 1;
            world
                .resource_mut::<Time<Real>>()
                .advance_by(std::time::Duration::from_secs(30));
            app.update();
        }
        // A healthy new snapshot clears loss; normal explosion behaviour resumes.
        app.world_mut()
            .resource_mut::<GameState>()
            .submarine
            .as_mut()
            .unwrap()
            .common
            .damage
            .destroyed = false;
        app.update();
        let world = app.world_mut();
        let mut query = world.query_filtered::<(&Text, &Visibility), With<ExplosionBanner>>();
        let (text, visibility) = query.single(world).unwrap();
        assert_eq!(*visibility, Visibility::Hidden);
        assert_ne!(text.0, "SOUS-MARIN PERDU");
    }

    #[test]
    fn banner_is_unique_non_interactive_and_initially_hidden() {
        let mut app = App::new();
        app.add_systems(Startup, spawn_banner);
        app.update();
        let world = app.world_mut();
        let mut query =
            world.query_filtered::<(&Pickable, &Visibility, &Node), With<ExplosionBanner>>();
        let (pickable, visibility, node) = query.single(world).unwrap();
        assert!(!pickable.should_block_lower);
        assert!(!pickable.is_hoverable);
        assert_eq!(*visibility, Visibility::Hidden);
        assert_eq!(node.height, px(32));
    }
}
