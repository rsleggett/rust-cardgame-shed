//! Width-driven responsive tweaks for small screens (phones). The card table
//! itself scales via the camera (`ScalingMode::AutoMin`), but the screen-space
//! UI panels are sized in fixed logical pixels, so on a narrow canvas the
//! space-hungry ones overflow.
//!
//! Below the breakpoint the score HUD + rules panel are pulled off-screen and
//! folded into the phone info overlay (a "?" button → veil → both panels +
//! close button); above it they sit inline and the overlay controls hide. This
//! one system is the single source of truth for which of those widgets is
//! visible (and their z-order), so the rules and behaviour can't drift apart.

use bevy::prelude::*;

use crate::ui::info_overlay::{InfoButton, InfoCloseButton, InfoOverlayVeil, InfoPanelOpen};
use crate::ui::rules_panel::RulesInfoPanel;
use crate::ui::score_hud::ScoreHud;

/// Below this logical width (px) we treat the canvas as a phone-sized screen:
/// the score HUD + rules legend (each several hundred px wide) move into the
/// info overlay instead of spilling across the narrow canvas. Landscape phones,
/// tablets, and desktops stay above the breakpoint and keep the panels inline.
const RULES_PANEL_MIN_WIDTH: f32 = 760.0;

/// Global z-layers for the overlay stack (only relevant while narrow + open).
const Z_VEIL: i32 = 50;
const Z_PANEL: i32 = 70;

/// Toggles the score HUD, rules panel, and info-overlay controls based on the
/// window width and whether the overlay is open. Writes only when a value
/// actually changes, so it doesn't dirty change-detection every frame.
#[allow(clippy::type_complexity)]
pub(crate) fn apply_responsive_layout(
    windows: Query<&Window>,
    mut info_open: ResMut<InfoPanelOpen>,
    mut q: Query<
        (
            &mut Visibility,
            &mut ZIndex,
            Has<ScoreHud>,
            Has<RulesInfoPanel>,
            Has<InfoButton>,
            Has<InfoOverlayVeil>,
            Has<InfoCloseButton>,
        ),
        Or<(
            With<ScoreHud>,
            With<RulesInfoPanel>,
            With<InfoButton>,
            With<InfoOverlayVeil>,
            With<InfoCloseButton>,
        )>,
    >,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let narrow = window.width() < RULES_PANEL_MIN_WIDTH;

    // Wide screens never use the overlay — force it shut so it can't linger when
    // the window is resized back up.
    if !narrow && info_open.0 {
        info_open.0 = false;
    }
    let open = info_open.0;

    for (mut vis, mut z, is_score, is_rules, is_info_btn, is_veil, is_close) in &mut q {
        let (want_vis, want_z) = if is_info_btn {
            // Opener: only on a narrow, closed overlay.
            (narrow && !open, ZIndex::Global(Z_VEIL + 10))
        } else if is_veil {
            (narrow && open, ZIndex::Global(Z_VEIL))
        } else if is_close {
            (narrow && open, ZIndex::Global(Z_PANEL + 10))
        } else if is_score || is_rules {
            // Panels: inline (normal local stacking) when wide, popped above the
            // veil when narrow + open.
            let visible = !narrow || open;
            let layer = if narrow { ZIndex::Global(Z_PANEL) } else { ZIndex::Local(0) };
            (visible, layer)
        } else {
            continue;
        };

        let desired_vis = if want_vis { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != desired_vis {
            *vis = desired_vis;
        }

        if !zindex_eq(&z, &want_z) {
            *z = want_z;
        }
    }
}

/// `ZIndex` doesn't derive `PartialEq`, so compare the variants by hand to keep
/// the system from dirtying change-detection (and re-laying-out the UI) every
/// frame.
fn zindex_eq(a: &ZIndex, b: &ZIndex) -> bool {
    match (a, b) {
        (ZIndex::Local(x), ZIndex::Local(y)) => x == y,
        (ZIndex::Global(x), ZIndex::Global(y)) => x == y,
        _ => false,
    }
}
