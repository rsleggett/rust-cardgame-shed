//! Background music: four looping stems (pad, bass, drums, drums_alt) mixed
//! dynamically by game phase, with a Ctrl+M mute toggle. All loops are the
//! same length and are started together in one frame (spawned paused,
//! un-paused once every stem has loaded — otherwise they start ~140ms apart),
//! so they stay phase-locked; after that only per-layer volume changes,
//! nothing is ever stopped or restarted. Asset load failure is non-fatal —
//! Bevy logs a warning per missing file and that layer stays silent. Runs on
//! native and web; on web the start waits for the first user gesture (see
//! `update_music_layers`).
//!
//! The original Ableton export had 3 drum-ish stems; two of them
//! (`drums_1`/`drums_2`) turned out to be silent/near-silent on bounce (-80dB
//! and -91dB respectively — verified with `ffmpeg -af volumedetect`, so it
//! wasn't a lossy OGG conversion issue), so only the third ("Bang Some Loop",
//! now just `drums`) is wired up. A re-exported second drum stem (`drums_alt`)
//! is wired up too — `update_music_layers` alternates which of the two is
//! audible every loop (~42.67s) during `Playing`, both spawned and looping the
//! whole time like every other layer, just muted/unmuted in turn.

use bevy::audio::{PlaybackSettings, Volume};
use bevy::prelude::*;

use crate::components::game::{GamePhase, GameState};

/// One stem in the dynamic music mix. Each variant maps to its own looping
/// OGG under `assets/music/stems/`.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MusicLayer {
    Pad,
    Bass,
    Drums,
    /// Alternate drum stem (re-exported from Ableton). Never audible at the
    /// same time as `Drums` — `update_music_layers` flips which one is "on"
    /// every loop, as an experiment to see how the alternation sounds.
    DrumsAlt,
}

impl MusicLayer {
    const ALL: [MusicLayer; 4] =
        [MusicLayer::Pad, MusicLayer::Bass, MusicLayer::Drums, MusicLayer::DrumsAlt];

    fn asset_path(self) -> &'static str {
        match self {
            MusicLayer::Pad => "music/stems/pad.ogg",
            MusicLayer::Bass => "music/stems/bass.ogg",
            MusicLayer::Drums => "music/stems/drums.ogg",
            MusicLayer::DrumsAlt => "music/stems/drums_alt.ogg",
        }
    }

    /// This layer's volume multiplier (0..=1, before `MUSIC_VOLUME`) for the
    /// given phase. `spectate` is only consulted during `Playing` — it's the
    /// endgame rush (human eliminated, AI resolving the rest of the round
    /// fast) and is when the mix leans hardest into bass + drums. Calm phases
    /// keep just the pad; a normal `Playing` round layers in bass + drums at a
    /// moderate level; `GameOver` fades back down to pad only.
    ///
    /// `drums_alt_active` picks which of `Drums`/`DrumsAlt` gets the "drums"
    /// slot this loop — the other gets silence — so exactly one drum stem is
    /// ever audible.
    fn target_volume(self, phase: GamePhase, spectate: bool, drums_alt_active: bool) -> f32 {
        use GamePhase::*;
        use MusicLayer::*;
        let drums_level = match phase {
            Dealing | Swap | Drafting | GameOver => 0.0,
            Playing if spectate => 0.85,
            Playing => 0.6,
        };
        match (phase, self) {
            (Dealing | Swap | Drafting, Pad) => 0.9,
            (Dealing | Swap | Drafting, Bass) => 0.0,
            (Playing, Pad) if spectate => 0.55,
            (Playing, Bass) if spectate => 0.85,
            (Playing, Pad) => 0.65,
            (Playing, Bass) => 0.75,
            (GameOver, Pad) => 0.7,
            (GameOver, Bass) => 0.0,
            (_, Drums) => {
                if drums_alt_active {
                    0.0
                } else {
                    drums_level
                }
            }
            (_, DrumsAlt) => {
                if drums_alt_active {
                    drums_level
                } else {
                    0.0
                }
            }
        }
    }

}

/// Measured loop length of every stem (`ffprobe` on the source AIFFs), used
/// to figure out when a loop has just rolled over so `Drums`/`DrumsAlt` can
/// swap which one is audible.
const LOOP_DURATION_SECS: f32 = 42.666_667;

/// Current mute state for the background music. Persists across mute toggles
/// so we restore the player's preference after a track restarts.
#[derive(Resource, Default)]
pub(crate) struct MusicMuted(pub(crate) bool);

pub(crate) const MUSIC_VOLUME: f32 = 0.35;

/// Per-second volume change allowed while easing a layer toward its target —
/// keeps phase transitions (and mute toggles) as smooth crossfades rather
/// than hard cuts.
const FADE_RATE: f32 = 0.5;

/// How long to wait for every stem to load before starting the ones that did
/// (a missing/corrupt file must not hold the whole soundtrack back forever).
const START_TIMEOUT_SECS: f32 = 5.0;

/// Spawns the stem sinks on startup, silent **and paused**. Bevy starts a
/// sink the moment its own asset finishes loading, and the files load on
/// different frames (measured: ~140ms apart), so un-paused stems would play
/// out of time with each other. `update_music_layers` instead un-pauses every
/// stem in the same frame once they've all loaded. Each OGG asset is optional
/// — if a file under `assets/music/stems/` is absent Bevy logs an asset-load
/// warning for that layer and the rest still play (after `START_TIMEOUT_SECS`).
pub(crate) fn setup_music(mut commands: Commands, asset_server: Res<AssetServer>) {
    for layer in MusicLayer::ALL {
        commands.spawn((
            layer,
            AudioBundle {
                source: asset_server.load(layer.asset_path()),
                settings: PlaybackSettings::LOOP.paused().with_volume(Volume::new(0.0)),
            },
        ));
    }
}

/// Eases each stem's volume toward its phase-driven target every frame.
/// Muting drives every target to zero, which — thanks to the same easing —
/// reads as a fade-out rather than an abrupt cut.
pub(crate) fn update_music_layers(
    game_state: Res<GameState>,
    muted: Res<MusicMuted>,
    time: Res<Time>,
    sinks: Query<(&MusicLayer, &AudioSink)>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mut started_at: Local<Option<f32>>,
    mut gesture_seen: Local<bool>,
) {
    // Browsers keep the AudioContext suspended until the first user gesture
    // (index.html resumes it on that same input). Hold the synchronized start
    // until then on web, so the audio and the loop clock below begin together
    // instead of the clock running ahead of silent playback. Native starts
    // immediately.
    if cfg!(not(target_arch = "wasm32"))
        || keys.get_just_pressed().next().is_some()
        || mouse.get_just_pressed().next().is_some()
        || touches.iter_just_pressed().next().is_some()
    {
        *gesture_seen = true;
    }

    // Stems are spawned paused; start them all in one frame (see
    // `setup_music`) so they're sample-aligned. The start time doubles as the
    // shared loop clock below.
    let now = time.elapsed_seconds();
    let started = match *started_at {
        Some(t) => t,
        None => {
            let ready = sinks.iter().count();
            if !*gesture_seen || (ready < MusicLayer::ALL.len() && now < START_TIMEOUT_SECS) {
                return;
            }
            for (_, sink) in &sinks {
                sink.play();
            }
            *started_at = Some(now);
            now
        }
    };

    // An odd loop count flips which drum stem is audible.
    let loop_count = ((now - started) / LOOP_DURATION_SECS).floor() as i64;
    let drums_alt_active = loop_count % 2 != 0;

    let max_delta = FADE_RATE * time.delta_seconds();
    for (layer, sink) in &sinks {
        let target = if muted.0 {
            0.0
        } else {
            layer.target_volume(game_state.phase, game_state.spectate_mode, drums_alt_active)
                * MUSIC_VOLUME
        };
        let current = sink.volume();
        let next = (target - current).clamp(-max_delta, max_delta) + current;
        if (next - current).abs() > f32::EPSILON {
            sink.set_volume(next);
        }
    }
}

/// Ctrl+M toggles background-music mute. Bound under a modifier so the bare
/// M key continues to consume Mulligan during play without ambiguity.
pub(crate) fn toggle_music_mute(keys: Res<ButtonInput<KeyCode>>, mut muted: ResMut<MusicMuted>) {
    let ctrl_held = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if !ctrl_held || !keys.just_pressed(KeyCode::KeyM) {
        return;
    }
    muted.0 = !muted.0;
}
