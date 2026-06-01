use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use shed::game_plugin::GamePlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Shed".to_string(),
                        resolution: (1440.0, 900.0).into(),
                        // Web-only fields (ignored on native): bind to the <canvas>
                        // in index.html, size it to the page, and keep game keys
                        // from triggering browser shortcuts.
                        canvas: Some("#bevy-canvas".to_string()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                })
                // On wasm there are no `.meta` sidecar files; without this Bevy
                // probes for them, the dev server answers with index.html, and the
                // failed parse aborts the asset load (blank fonts). Skip the probe.
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .add_plugins(GamePlugin)
        .run();
}
