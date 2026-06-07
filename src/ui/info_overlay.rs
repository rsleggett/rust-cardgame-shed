//! Phone/portrait info overlay. On a narrow canvas the always-on score HUD and
//! rules panel are hidden (they crowd the small screen); a small "?" button in
//! the bottom-left instead pops both panels up over a dark veil with a close
//! button. On wide screens the panels stay inline and these controls hide.
//!
//! The actual show/hide + z-ordering is driven centrally by
//! `responsive::apply_responsive_layout`, which reads `InfoPanelOpen` and the
//! window width. This module only owns the resource, the markers, the spawn,
//! and the open/close click handler.

use bevy::prelude::*;

use crate::theme;

/// Whether the phone info overlay is currently open. Only meaningful on a narrow
/// canvas; ignored (and forced shut) on wide screens.
#[derive(Resource, Default)]
pub(crate) struct InfoPanelOpen(pub(crate) bool);

/// Bottom-left "?" button that opens the overlay (narrow screens only).
#[derive(Component)]
pub(crate) struct InfoButton;

/// Full-screen dark veil shown behind the panels while the overlay is open.
#[derive(Component)]
pub(crate) struct InfoOverlayVeil;

/// Top-left "X" button that closes the overlay.
#[derive(Component)]
pub(crate) struct InfoCloseButton;

/// Spawns the info button, the veil, and the close button. All start hidden;
/// `apply_responsive_layout` reveals the right ones based on width + open state.
pub(crate) fn spawn_info_controls(commands: &mut Commands, ui_font: Handle<Font>) {
    // Dark veil — full screen, behind the popped-up panels.
    commands.spawn((
        InfoOverlayVeil,
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: Color::srgba(0.02, 0.07, 0.05, 0.78).into(),
            visibility: Visibility::Hidden,
            z_index: ZIndex::Global(50),
            ..default()
        },
    ));

    // Bottom-left "?" opener.
    spawn_icon_button(commands, InfoButton, "?", theme::CYAN, ui_font.clone(), true, false);
    // Top-left "X" closer.
    spawn_icon_button(commands, InfoCloseButton, "X", theme::MAGENTA, ui_font, false, true);
}

/// Small square icon button, bottom-left (opener) or top-left (closer).
fn spawn_icon_button<M: Component>(
    commands: &mut Commands,
    marker: M,
    label: &str,
    fill: Color,
    font: Handle<Font>,
    bottom_anchored: bool,
    is_close: bool,
) {
    let mut style = Style {
        position_type: PositionType::Absolute,
        left: Val::Px(24.0),
        width: Val::Px(44.0),
        height: Val::Px(44.0),
        border: UiRect::all(Val::Px(2.0)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    };
    if bottom_anchored {
        style.bottom = Val::Px(24.0);
    } else {
        style.top = Val::Px(24.0);
    }

    commands
        .spawn((
            marker,
            ButtonBundle {
                style,
                background_color: fill.into(),
                border_color: theme::chunky_shadow(fill).into(),
                border_radius: BorderRadius::all(Val::Px(22.0)),
                visibility: Visibility::Hidden,
                z_index: ZIndex::Global(if is_close { 80 } else { 60 }),
                ..default()
            },
            Outline {
                width: Val::Px(2.0),
                offset: Val::Px(0.0),
                color: Color::srgba(1.0, 1.0, 1.0, 0.35),
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                label,
                TextStyle { font, font_size: 22.0, color: Color::srgb(0.10, 0.06, 0.10) },
            ));
        });
}

/// Opens the overlay on the "?" press, closes it on the "X" press.
pub(crate) fn handle_info_buttons(
    mut info_open: ResMut<InfoPanelOpen>,
    info_btn_q: Query<&Interaction, (Changed<Interaction>, With<InfoButton>)>,
    close_btn_q: Query<&Interaction, (Changed<Interaction>, With<InfoCloseButton>)>,
) {
    for interaction in &info_btn_q {
        if *interaction == Interaction::Pressed {
            info_open.0 = true;
        }
    }
    for interaction in &close_btn_q {
        if *interaction == Interaction::Pressed {
            info_open.0 = false;
        }
    }
}
