//! A deliberately non-directional cue: audibility must not disclose an explosion position.
use crate::{
    button_activation::{self, ButtonActivated},
    network::{GameState, NetworkReceiveSet},
};
use bevy::prelude::*;

#[derive(Resource)]
struct Cue {
    handle: Handle<AudioSource>,
    heard: u64,
    enabled: bool,
    last_played: f64,
}
#[derive(Component)]
struct SoundToggle;
#[derive(Component)]
struct SoundLabel;
#[derive(Component)]
struct ExplosionSound;

pub struct ExplosionAudioPlugin;
impl Plugin for ExplosionAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, update.after(NetworkReceiveSet));
    }
}

fn setup(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let handle = sources.add(AudioSource {
        bytes: explosion_wav().into(),
    });
    commands.insert_resource(Cue {
        handle,
        heard: 0,
        enabled: true,
        last_played: -1.0,
    });
    let mut button = commands.spawn((
        Button,
        SoundToggle,
        Visibility::Hidden,
        GlobalZIndex(101),
        Node {
            position_type: PositionType::Absolute,
            top: px(4),
            right: px(8),
            width: px(60),
            height: px(44),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.04, 0.18, 0.23)),
    ));
    button_activation::enable(&mut button);
    button.with_children(|p| {
        p.spawn((
            SoundLabel,
            Text::new("SON ON"),
            TextFont {
                font_size: FontSize::Px(11.0),
                ..default()
            },
            TextColor(Color::WHITE),
            Pickable::IGNORE,
        ));
    });
}

fn update(
    mut commands: Commands,
    state: Res<GameState>,
    time: Res<Time<Real>>,
    mut cue: ResMut<Cue>,
    mut clicks: MessageReader<ButtonActivated>,
    mut toggle: Single<(Entity, &mut Visibility), With<SoundToggle>>,
    mut label: Single<&mut Text, With<SoundLabel>>,
    sounds: Query<Entity, With<ExplosionSound>>,
) {
    *toggle.1 = if state.game_started {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for click in clicks.read() {
        if click.0 == toggle.0 && state.game_started {
            cue.enabled = !cue.enabled;
        }
    }
    label.0 = if cue.enabled { "SON ON" } else { "MUET" }.into();
    if !cue.enabled || !state.game_started {
        for entity in &sounds {
            commands.entity(entity).despawn();
        }
    }
    let incoming = state.audible_explosions > cue.heard;
    cue.heard = state.audible_explosions;
    if incoming
        && state.game_started
        && cue.enabled
        && sounds.iter().count() < 4
        && time.elapsed_secs_f64() - cue.last_played >= 0.15
    {
        commands.spawn((
            ExplosionSound,
            AudioPlayer::new(cue.handle.clone()),
            PlaybackSettings::DESPAWN,
        ));
        cue.last_played = time.elapsed_secs_f64();
    }
}

/// Embedded synthetic low-frequency blast. No asset download and no external sound licence.
fn explosion_wav() -> Vec<u8> {
    const RATE: u32 = 22_050;
    let count = RATE * 4 / 5;
    let bytes = count * 2;
    let mut wav = Vec::with_capacity(44 + bytes as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1_u16.to_le_bytes()); // mono, not a directional measurement
    wav.extend_from_slice(&RATE.to_le_bytes());
    wav.extend_from_slice(&(RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&bytes.to_le_bytes());
    let mut seed = 7_u32;
    let mut low = 0.0_f32;
    for i in 0..count {
        let t = i as f32 / RATE as f32;
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = seed as f32 / u32::MAX as f32 * 2.0 - 1.0;
        low += 0.08 * (noise - low);
        let envelope = (t * 100.0).min(1.0) * (-7.0 * t).exp() * (1.0 - t / 0.8).max(0.0);
        let sample = ((low * 0.7 + (t * 65.0 * std::f32::consts::TAU).sin() * 0.3)
            * envelope
            * 16000.0) as i16;
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_blast_is_a_bounded_mono_pcm_wav() {
        let wav = explosion_wav();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + 22_050 * 4 / 5 * 2);
        assert!(wav[44..].iter().any(|b| *b != 0));
        assert_eq!(&wav[wav.len() - 2..], &[0, 0]);
    }
}
