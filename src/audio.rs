//! Background music: a single looping OGG track with a Ctrl+M mute toggle.
//! Asset load failure is non-fatal — Bevy logs a warning and the game runs
//! silently. See scripts/download-music.sh.

#[cfg(not(target_arch = "wasm32"))]
use bevy::audio::{PlaybackSettings, Volume};
use bevy::prelude::*;

/// Marker on the single looping background-music entity. Used by the mute
/// toggle to find the audio sink.
#[derive(Component)]
pub(crate) struct BackgroundMusic;

/// Current mute state for the background music. Persists across mute toggles
/// so we restore the player's preference after a track restarts.
#[derive(Resource, Default)]
pub(crate) struct MusicMuted(pub(crate) bool);

pub(crate) const MUSIC_VOLUME: f32 = 0.35;

/// Spawns the background music sink on startup. The OGG asset is optional —
/// if `assets/music/lofi_loop.ogg` is absent Bevy logs an asset-load warning
/// and the game runs silently. See scripts/download-music.sh.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn setup_music(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        BackgroundMusic,
        AudioBundle {
            source: asset_server.load("music/lofi_loop.ogg"),
            settings: PlaybackSettings::LOOP.with_volume(Volume::new(MUSIC_VOLUME)),
        },
    ));
}

/// The web build ships without a bundled track. Crucially, a missing OGG on a
/// static host is served as the SPA fallback (HTML), which `bevy_audio` then
/// tries to decode and panics on (`UnrecognizedFormat`). So on wasm we skip
/// music entirely — the game runs silent. (Native still loads the track above.)
#[cfg(target_arch = "wasm32")]
pub(crate) fn setup_music() {}

/// Ctrl+M toggles background-music mute. Bound under a modifier so the bare
/// M key continues to consume Mulligan during play without ambiguity.
pub(crate) fn toggle_music_mute(
    keys: Res<ButtonInput<KeyCode>>,
    mut muted: ResMut<MusicMuted>,
    sinks: Query<&AudioSink, With<BackgroundMusic>>,
) {
    let ctrl_held = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if !ctrl_held || !keys.just_pressed(KeyCode::KeyM) {
        return;
    }
    muted.0 = !muted.0;
    if let Ok(sink) = sinks.get_single() {
        sink.set_volume(if muted.0 { 0.0 } else { MUSIC_VOLUME });
    }
}
