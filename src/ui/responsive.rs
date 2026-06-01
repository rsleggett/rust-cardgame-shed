//! Width-driven responsive tweaks for small screens (phones). The card table
//! itself scales via the camera (`ScalingMode::AutoMin`), but the screen-space
//! UI panels are sized in fixed logical pixels, so on a narrow canvas the
//! space-hungry ones overflow. This hides the least-essential panel below a
//! width breakpoint.

use bevy::prelude::*;

use crate::ui::rules_panel::RulesInfoPanel;

/// Below this logical width (px) we treat the canvas as a phone-sized screen
/// and hide the bottom-left rules legend, which is ~400px wide and otherwise
/// spills across a narrow portrait canvas. Landscape phones, tablets, and
/// desktops stay above the breakpoint and keep the panel.
const RULES_PANEL_MIN_WIDTH: f32 = 760.0;

/// Toggles screen-space UI based on the current window width. Cheap: a single
/// window read plus a visibility compare, and it only writes when the state
/// actually flips (so it doesn't dirty change-detection every frame).
pub(crate) fn apply_responsive_layout(
    windows: Query<&Window>,
    mut rules_panel: Query<&mut Visibility, With<RulesInfoPanel>>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let wide_enough = window.width() >= RULES_PANEL_MIN_WIDTH;
    let desired = if wide_enough {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut rules_panel {
        if *visibility != desired {
            *visibility = desired;
        }
    }
}
