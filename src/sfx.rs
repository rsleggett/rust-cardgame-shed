//! Sound effects: a small bank of one-shot clips loaded at startup and fired by
//! gameplay events. Like the background music, every clip is **optional** — a
//! missing file just means that effect is silent (Bevy logs an asset-load
//! warning, no crash). See `scripts/download-sfx.sh`.
//!
//! Clips are WAV (Kenney's CC0 "Interface Sounds" ship as WAV); the `wav` Bevy
//! feature is enabled in `Cargo.toml` to decode them.
//!
//! Most cues are derived purely from `GameState` deltas (`sfx_director`) so the
//! gameplay code stays untouched — the same delta-watch pattern as the visual
//! `detect_juice_events`. Button presses and invalid-card clicks have their own
//! tiny hook systems. SFX are independent of `ReducedMotion` (that gates motion,
//! not audio).

use bevy::prelude::*;

use crate::components::game::{GamePhase, GameState};
use crate::systems::input::InvalidCardClicked;

/// One-shot SFX playback volume.
pub(crate) const SFX_VOLUME: f32 = 0.5;

/// The distinct sound effects in the bank.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SfxKind {
    CardPlay,
    Burn,
    Pickup,
    Deal,
    Button,
    Score,
    Invalid,
}

/// Request to play a one-shot effect. Written by `sfx_director` and the hook
/// systems; read by `play_sfx`.
#[derive(Event)]
pub struct SfxEvent(pub SfxKind);

/// Handles for the loaded clips. Missing files resolve to a failed-load handle
/// that simply never plays — graceful-missing, like the background music.
#[derive(Resource)]
pub struct Sfx {
    card_play: Handle<AudioSource>,
    burn: Handle<AudioSource>,
    pickup: Handle<AudioSource>,
    deal: Handle<AudioSource>,
    button: Handle<AudioSource>,
    score: Handle<AudioSource>,
    invalid: Handle<AudioSource>,
}

impl Sfx {
    fn handle(&self, kind: SfxKind) -> Handle<AudioSource> {
        match kind {
            SfxKind::CardPlay => self.card_play.clone(),
            SfxKind::Burn => self.burn.clone(),
            SfxKind::Pickup => self.pickup.clone(),
            SfxKind::Deal => self.deal.clone(),
            SfxKind::Button => self.button.clone(),
            SfxKind::Score => self.score.clone(),
            SfxKind::Invalid => self.invalid.clone(),
        }
    }
}

/// Loads the SFX bank on startup. Runs on both native and web: the clips are
/// bundled into the web build too (CI runs `scripts/download-sfx.sh` before
/// `trunk build`, and `index.html` copies `assets/` into `dist/`). A missing
/// clip stays graceful on native; on a static host the files must actually be
/// present (CI guarantees this) — otherwise the decoder chokes on the SPA
/// fallback. The browser autoplay policy is handled by the AudioContext-resume
/// snippet in `index.html`.
pub(crate) fn setup_sfx(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Sfx {
        card_play: asset_server.load("sfx/card_play.wav"),
        burn: asset_server.load("sfx/burn.wav"),
        pickup: asset_server.load("sfx/pickup.wav"),
        deal: asset_server.load("sfx/deal.wav"),
        button: asset_server.load("sfx/button.wav"),
        score: asset_server.load("sfx/score.wav"),
        invalid: asset_server.load("sfx/invalid.wav"),
    });
}

/// Spawns a one-shot audio entity per `SfxEvent`, despawning itself when the
/// clip finishes. Runs on native and web alike.
pub(crate) fn play_sfx(
    mut commands: Commands,
    mut events: EventReader<SfxEvent>,
    sfx: Res<Sfx>,
) {
    use bevy::audio::{PlaybackSettings, Volume};
    for SfxEvent(kind) in events.read() {
        commands.spawn(AudioBundle {
            source: sfx.handle(*kind),
            settings: PlaybackSettings::DESPAWN.with_volume(Volume::new(SFX_VOLUME)),
        });
    }
}

/// Derives play / burn / pickup / score / deal cues from `GameState` deltas,
/// mirroring the visual `detect_juice_events`. Discriminators:
/// - pile grew → a card was played (`CardPlay`);
/// - the whole pile emptied to the discard → `Burn`;
/// - `needs_to_pickup` cleared mid-play → `Pickup` (covers Half Pickup, where
///   some cards also hit the discard — the pickup cue wins over the burn cue);
/// - a new finisher → `Score`; entering the Dealing phase → `Deal` (one shuffle
///   cue, not a per-card riffle).
#[allow(clippy::too_many_arguments)]
pub(crate) fn sfx_director(
    game_state: Res<GameState>,
    mut ev: EventWriter<SfxEvent>,
    mut last_in_play: Local<usize>,
    mut last_discard: Local<usize>,
    mut last_finished: Local<usize>,
    mut last_needs: Local<bool>,
    mut last_phase: Local<Option<GamePhase>>,
) {
    let in_play = game_state.cards_in_play.len();
    let discard = game_state.discard_pile.len();
    let finished = game_state.finish_order.len();
    let needs = game_state.needs_to_pickup;
    let phase = game_state.phase;

    // Shuffle cue when a new deal begins.
    if *last_phase != Some(phase) {
        if phase == GamePhase::Dealing {
            ev.send(SfxEvent(SfxKind::Deal));
        }
        *last_phase = Some(phase);
    }

    let pickup = *last_needs && !needs && phase == GamePhase::Playing;
    if pickup {
        ev.send(SfxEvent(SfxKind::Pickup));
    } else if discard > *last_discard {
        ev.send(SfxEvent(SfxKind::Burn));
    } else if in_play > *last_in_play {
        ev.send(SfxEvent(SfxKind::CardPlay));
    }
    if finished > *last_finished {
        ev.send(SfxEvent(SfxKind::Score));
    }

    *last_in_play = in_play;
    *last_discard = discard;
    *last_finished = finished;
    *last_needs = needs;
}

/// Any chunky/UI button press → a click cue.
pub(crate) fn button_sfx(
    q: Query<&Interaction, (Changed<Interaction>, With<Button>)>,
    mut ev: EventWriter<SfxEvent>,
) {
    for interaction in &q {
        if *interaction == Interaction::Pressed {
            ev.send(SfxEvent(SfxKind::Button));
        }
    }
}

/// An illegal card click (also drives the red flash) → a buzz.
pub(crate) fn invalid_sfx(
    mut reader: EventReader<InvalidCardClicked>,
    mut ev: EventWriter<SfxEvent>,
) {
    for _ in reader.read() {
        ev.send(SfxEvent(SfxKind::Invalid));
    }
}
