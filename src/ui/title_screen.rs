//! Title / main-menu screen (the handoff's `TitleA`, minimal version): the SHED
//! logo, tagline, a fan of four glowing hero cards, a PLAY button, and two
//! settings toggles (music mute, reduced motion). The Ante Ladder buttons from
//! the design (Continue Run / Daily / Collection) wait on that meta-system.
//!
//! The table is still built at startup underneath; a full-screen opaque
//! overlay hides it, and every gameplay system is gated on `AppState::Game`
//! (see `game_plugin.rs`), so nothing deals or ticks until PLAY is pressed.
//! The press doubles as the first user gesture the browser needs to start
//! audio on the web build.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::audio::MusicMuted;
use crate::components::card::Rank;
use crate::rendering::card_renderer::ReducedMotion;
use crate::theme;

/// Top-level app flow. The game systems only run in `Game`.
#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AppState {
    #[default]
    Title,
    Game,
}

/// Below this logical window width the title shrinks to phone proportions
/// (matches the rules-panel breakpoint in `responsive.rs`).
const COMPACT_WIDTH: f32 = 760.0;
const COMPACT_SCALE: f32 = 0.68;

/// Above every other UI layer (header 45, info overlay 50–80).
const Z_TITLE: i32 = 10_000;

#[derive(Component)]
pub(crate) struct TitleScreen;

/// Which action a title button performs.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TitleButton {
    Play,
    Music,
    Motion,
}

/// A button's resting fill, so hover/press can restyle and restore it.
#[derive(Component)]
pub(crate) struct TitleButtonFill(Color);

/// A text label that mirrors a setting; refreshed whenever the setting moves.
#[derive(Component)]
pub(crate) struct TitleLabel(TitleButton);

/// Font size at desktop scale; multiplied by the compact factor on phones.
#[derive(Component)]
pub(crate) struct TitleText(f32);

/// Fixed box dimensions / offsets at desktop scale, scaled like `TitleText`.
#[derive(Component, Default, Clone, Copy)]
pub(crate) struct TitleBox {
    w: Option<f32>,
    h: Option<f32>,
    left: Option<f32>,
    top: Option<f32>,
}

/// A hero card in the fan; `rot_deg` is its resting tilt (CSS-style, clockwise
/// positive). The idle wobble is added on top unless motion is reduced.
#[derive(Component)]
pub(crate) struct TitleCard {
    rot_deg: f32,
    phase: f32,
}

const MAGENTA_GLOW: Color = Color::srgba(1.0, 0.243, 0.604, 0.35);
const BUTTON_INK: Color = Color::srgb(0.10, 0.06, 0.10);

pub(crate) fn spawn_title_screen(mut commands: Commands, asset_server: Res<AssetServer>) {
    let ui_font = asset_server.load("fonts/Rubik-Regular.ttf");
    let pixel_font = asset_server.load("fonts/Silkscreen-Regular.ttf");
    let rank_font = asset_server.load("fonts/TitanOne-Regular.ttf");
    let suit_font = asset_server.load("fonts/NotoSansSymbols2-Regular.ttf");

    commands
        .spawn((
            TitleScreen,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                background_color: theme::FELT_DARK.into(),
                z_index: ZIndex::Global(Z_TITLE),
                ..default()
            },
        ))
        .with_children(|root| {
            // Gold inset frame, matching the table's.
            root.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(14.0),
                    right: Val::Px(14.0),
                    top: Val::Px(14.0),
                    bottom: Val::Px(14.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                border_color: theme::GOLD.with_alpha(0.35).into(),
                border_radius: BorderRadius::all(Val::Px(18.0)),
                ..default()
            });

            // ── Logo: white pixel text over two magenta drop-shadow layers ──
            // The wrapper sizes to the (in-flow) white text; the shadow layers
            // are absolute children offset behind it.
            root.spawn(NodeBundle {
                transform: Transform::from_rotation(Quat::from_rotation_z(2.0_f32.to_radians())),
                ..default()
            })
            .with_children(|logo| {
                for (offset, color, in_flow) in [
                    (7.0, MAGENTA_GLOW, false),
                    (4.0, theme::MAGENTA, false),
                    (0.0, theme::WHITE, true),
                ] {
                    let mut text = logo.spawn((
                        TitleText(112.0),
                        TextBundle::from_section(
                            "SHED",
                            TextStyle { font: pixel_font.clone(), font_size: 112.0, color },
                        ),
                    ));
                    if !in_flow {
                        text.insert((
                            TitleBox { left: Some(offset), top: Some(offset), ..default() },
                            Style {
                                position_type: PositionType::Absolute,
                                left: Val::Px(offset),
                                top: Val::Px(offset),
                                ..default()
                            },
                        ));
                    }
                }
            });

            root.spawn((
                TitleText(17.0),
                TextBundle::from_section(
                    "R O G U E L I K E   C A R D   G A M E",
                    TextStyle { font: ui_font.clone(), font_size: 17.0, color: theme::CYAN },
                )
                .with_style(Style { margin: UiRect::top(Val::Px(6.0)), ..default() }),
            ));

            // ── Hero card fan ──
            root.spawn((
                TitleBox { w: Some(330.0), h: Some(190.0), ..default() },
                NodeBundle {
                    style: Style { margin: UiRect::vertical(Val::Px(30.0)), ..default() },
                    ..default()
                },
            ))
            .with_children(|fan| {
                // (rank, glyph, red?, left, top, tilt°) — the handoff's fan, ×1.4 in
                // size but spread wider so every rank stays readable.
                let cards = [
                    (Rank::Ace, "A", "\u{2660}", false, 14.0, 34.0, -7.0),
                    (Rank::Two, "2", "\u{2665}", true, 82.0, 17.0, 3.5),
                    (Rank::Ten, "10", "\u{2663}", false, 150.0, 8.0, 14.0),
                    (Rank::Seven, "7", "\u{2666}", true, 218.0, 22.0, 10.5),
                ];
                for (i, (rank, label, suit, red, left, top, tilt)) in cards.into_iter().enumerate() {
                    spawn_hero_card(
                        fan, rank, label, suit, red, left, top, tilt, i as f32,
                        rank_font.clone(), suit_font.clone(), pixel_font.clone(),
                    );
                }
            });

            // ── Buttons ──
            spawn_button(
                root, TitleButton::Play, "PLAY", theme::GOLD, 340.0, 72.0, 34.0, ui_font.clone(),
            );
            root.spawn(NodeBundle {
                style: Style {
                    column_gap: Val::Px(12.0),
                    margin: UiRect::top(Val::Px(14.0)),
                    ..default()
                },
                ..default()
            })
            .with_children(|row| {
                spawn_button(row, TitleButton::Music, "", theme::CYAN, 164.0, 52.0, 16.0, ui_font.clone());
                spawn_button(row, TitleButton::Motion, "", theme::CYAN, 164.0, 52.0, 16.0, ui_font.clone());
            });

            root.spawn((
                TitleText(12.0),
                TextBundle::from_section(
                    format!("v{} \u{b7} ENTER OR CLICK TO PLAY", env!("CARGO_PKG_VERSION")),
                    TextStyle { font: pixel_font, font_size: 12.0, color: theme::WHITE.with_alpha(0.5) },
                )
                .with_style(Style { margin: UiRect::top(Val::Px(28.0)), ..default() }),
            ));
        });
}

#[allow(clippy::too_many_arguments)]
fn spawn_hero_card(
    fan: &mut ChildBuilder,
    rank: Rank,
    label: &str,
    suit: &str,
    red: bool,
    left: f32,
    top: f32,
    tilt_deg: f32,
    phase: f32,
    rank_font: Handle<Font>,
    suit_font: Handle<Font>,
    pixel_font: Handle<Font>,
) {
    let ink = theme::card_text_color(red);
    let glow = theme::special_color(rank);
    let mut card = fan.spawn((
        TitleCard { rot_deg: tilt_deg, phase },
        TitleBox { w: Some(88.0), h: Some(124.0), left: Some(left), top: Some(top) },
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(left),
                top: Val::Px(top),
                width: Val::Px(88.0),
                height: Val::Px(124.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::top(Val::Px(8.0)),
                ..default()
            },
            background_color: theme::CARD_PAPER.into(),
            border_radius: BorderRadius::all(Val::Px(9.0)),
            transform: Transform::from_rotation(Quat::from_rotation_z((-tilt_deg).to_radians())),
            ..default()
        },
    ));
    if let Some(color) = glow {
        card.insert(Outline {
            width: Val::Px(3.0),
            offset: Val::Px(1.0),
            color: color.with_alpha(0.9),
        });
    }
    card.with_children(|c| {
        c.spawn((
            TitleText(36.0),
            TextBundle::from_section(
                label,
                TextStyle { font: rank_font, font_size: 36.0, color: ink },
            ),
        ));
        c.spawn((
            TitleText(40.0),
            TextBundle::from_section(
                suit,
                TextStyle { font: suit_font, font_size: 40.0, color: ink },
            ),
        ));
        if let (Some(color), Some(badge)) = (glow, theme::special_badge(rank)) {
            c.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(7.0),
                    padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                    ..default()
                },
                background_color: color.into(),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            })
            .with_children(|b| {
                b.spawn((
                    TitleText(10.0),
                    TextBundle::from_section(
                        badge,
                        TextStyle { font: pixel_font, font_size: 10.0, color: BUTTON_INK },
                    ),
                ));
            });
        }
    });
}

/// A chunky title button (same look as the in-game ones: bright fill, darker
/// solid bottom edge, translucent rim) in normal flow rather than pinned to the
/// bottom, so it carries its own `TitleButton` marker and press styling.
#[allow(clippy::too_many_arguments)]
fn spawn_button(
    parent: &mut ChildBuilder,
    kind: TitleButton,
    label: &str,
    fill: Color,
    width: f32,
    height: f32,
    font_size: f32,
    font: Handle<Font>,
) {
    parent
        .spawn((
            kind,
            TitleButtonFill(fill),
            TitleBox { w: Some(width), h: Some(height), ..default() },
            ButtonBundle {
                style: Style {
                    width: Val::Px(width),
                    height: Val::Px(height),
                    border: UiRect::bottom(Val::Px(5.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                background_color: fill.into(),
                border_color: theme::chunky_shadow(fill).into(),
                border_radius: BorderRadius::all(Val::Px(14.0)),
                ..default()
            },
            Outline {
                width: Val::Px(2.0),
                offset: Val::Px(0.0),
                color: Color::srgba(1.0, 1.0, 1.0, 0.35),
            },
        ))
        .with_children(|b| {
            b.spawn((
                TitleLabel(kind),
                TitleText(font_size),
                TextBundle::from_section(
                    label,
                    TextStyle { font, font_size, color: BUTTON_INK },
                ),
            ));
        });
}

/// Hover lightens the fill; press slides the button down onto its shadow edge.
#[allow(clippy::type_complexity)]
pub(crate) fn style_title_buttons(
    mut q: Query<
        (&Interaction, &TitleButtonFill, &mut BackgroundColor, &mut BorderColor, &mut Style),
        (Changed<Interaction>, With<TitleButton>),
    >,
) {
    for (interaction, fill, mut bg, mut border, mut style) in &mut q {
        let (color, pressed) = match interaction {
            Interaction::Pressed => (fill.0, true),
            Interaction::Hovered => (theme::lighten(fill.0, 0.25), false),
            Interaction::None => (fill.0, false),
        };
        *bg = color.into();
        *border = theme::chunky_shadow(fill.0).into();
        style.top = Val::Px(if pressed { 5.0 } else { 0.0 });
        style.border = UiRect::bottom(Val::Px(if pressed { 0.0 } else { 5.0 }));
    }
}

/// Button actions. PLAY enters the game; the toggles flip their setting.
pub(crate) fn handle_title_buttons(
    q: Query<(&Interaction, &TitleButton), Changed<Interaction>>,
    mut next: ResMut<NextState<AppState>>,
    mut muted: ResMut<MusicMuted>,
    mut reduced: ResMut<ReducedMotion>,
) {
    for (interaction, kind) in &q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match kind {
            TitleButton::Play => next.set(AppState::Game),
            TitleButton::Music => muted.0 = !muted.0,
            TitleButton::Motion => reduced.0 = !reduced.0,
        }
    }
}

/// Enter / Space also starts the game.
pub(crate) fn title_start_key(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if keys.any_just_pressed([KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space]) {
        next.set(AppState::Game);
    }
}

/// Keeps the toggle labels in sync with the settings (also covers the Ctrl+M /
/// Ctrl+R shortcuts, which flip the same resources).
pub(crate) fn update_title_labels(
    muted: Res<MusicMuted>,
    reduced: Res<ReducedMotion>,
    mut q: Query<(&TitleLabel, &mut Text)>,
) {
    for (label, mut text) in &mut q {
        let s = match label.0 {
            TitleButton::Play => "PLAY",
            TitleButton::Music => if muted.0 { "MUSIC: OFF" } else { "MUSIC: ON" },
            TitleButton::Motion => if reduced.0 { "MOTION: LOW" } else { "MOTION: FULL" },
        };
        if text.sections[0].value != s {
            text.sections[0].value = s.to_string();
        }
    }
}

/// Re-sizes the title for the current window: full size on desktop, shrunk on
/// phone-width canvases. Only does work when the scale actually changes.
#[allow(clippy::type_complexity)]
pub(crate) fn scale_title(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut last: Local<f32>,
    mut texts: Query<(&TitleText, &mut Text)>,
    mut boxes: Query<(&TitleBox, &mut Style)>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let scale = if window.width() < COMPACT_WIDTH { COMPACT_SCALE } else { 1.0 };
    if (*last - scale).abs() < f32::EPSILON {
        return;
    }
    *last = scale;
    for (base, mut text) in &mut texts {
        for section in &mut text.sections {
            section.style.font_size = base.0 * scale;
        }
    }
    for (b, mut style) in &mut boxes {
        if let Some(w) = b.w {
            style.width = Val::Px(w * scale);
        }
        if let Some(h) = b.h {
            style.height = Val::Px(h * scale);
        }
        if let Some(l) = b.left {
            style.left = Val::Px(l * scale);
        }
        if let Some(t) = b.top {
            style.top = Val::Px(t * scale);
        }
    }
}

/// A slow wobble on the hero cards. Skipped under reduced motion (cards sit at
/// their resting tilt).
pub(crate) fn animate_title_cards(
    time: Res<Time>,
    reduced: Res<ReducedMotion>,
    mut q: Query<(&TitleCard, &mut Transform)>,
) {
    let t = time.elapsed_seconds();
    for (card, mut tf) in &mut q {
        let wobble = if reduced.0 { 0.0 } else { (t * 1.3 + card.phase * 1.7).sin() * 1.8 };
        tf.rotation = Quat::from_rotation_z((-(card.rot_deg + wobble)).to_radians());
    }
}

pub(crate) fn despawn_title_screen(mut commands: Commands, q: Query<Entity, With<TitleScreen>>) {
    for e in &q {
        commands.entity(e).despawn_recursive();
    }
}
