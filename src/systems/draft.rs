//! Buff draft phase: between rounds each seat picks one perk. Pool generation
//! filters out kinds the seat already owns; previous-round Shed gets a bigger
//! pool. Human picks via overlay click, AIs auto-pick randomly.

use bevy::prelude::*;

use crate::components::game::{
    ActiveBuff, BuffKind, GamePhase, GameState, MatchState,
};
use crate::theme;

/// Near-black solid fill for a draft row. Keeps the perk text legible (light on
/// dark) regardless of the cards showing through the veil behind, while the
/// rarity colour stays as the row's border + accent.
const ITEM_BG: Color = Color::srgb(0.043, 0.063, 0.078);

/// Marker on the full-screen draft overlay (one per round).
#[derive(Component)]
pub(crate) struct DraftScreen;

/// Marker on each clickable buff row inside the draft overlay.
#[derive(Component)]
pub(crate) struct DraftOption(pub(crate) BuffKind);

/// Per-round draft state: one pool per seat, one optional pick per seat.
/// Re-populated by `setup_draft_system` whenever phase enters Drafting.
#[derive(Resource, Default)]
pub(crate) struct DraftState {
    pub pools: Vec<Vec<BuffKind>>,
    pub picks: Vec<Option<BuffKind>>,
}

impl DraftState {
    fn reset(&mut self, player_count: usize) {
        self.pools.clear();
        self.pools.resize_with(player_count, Vec::new);
        self.picks.clear();
        self.picks.resize(player_count, None);
    }

    fn all_picked(&self) -> bool {
        !self.picks.is_empty() && self.picks.iter().all(Option::is_some)
    }
}

// ── draft systems ─────────────────────────────────────────────────────────────

/// One-time per round: populate the draft pools as soon as phase enters
/// Drafting. Idempotent — the `pools.is_empty()` guard keeps it from re-running
/// every frame while the human is reading their options.
pub(crate) fn setup_draft_system(
    mut draft_state: ResMut<DraftState>,
    game_state: Res<GameState>,
    match_state: Res<MatchState>,
) {
    if game_state.phase != GamePhase::Drafting {
        return;
    }
    if !draft_state.pools.is_empty() {
        return;
    }
    draft_state.reset(game_state.players.len());
    for seat in 0..game_state.players.len() {
        // Previous round's Shed gets a bigger pool; everyone else gets 3.
        let size = if match_state.previous_shed == Some(seat) { 5 } else { 3 };
        draft_state.pools[seat] = roll_pool(&game_state.players[seat].modifiers, size);
    }
}

/// Pick `size` distinct BuffKinds at random, excluding kinds the player already
/// owns. Falls back gracefully (returns fewer kinds) once the catalogue is
/// exhausted — rare in a 5-round match with 8 buffs.
fn roll_pool(owned: &[ActiveBuff], size: usize) -> Vec<BuffKind> {
    let mut available: Vec<BuffKind> = BuffKind::ALL
        .iter()
        .copied()
        .filter(|k| !owned.iter().any(|b| b.kind == *k))
        .collect();
    // Fisher-Yates using the same rand source the rest of the project uses.
    for i in (1..available.len()).rev() {
        let j = (rand::random::<f32>() * (i + 1) as f32) as usize;
        if j <= i {
            available.swap(i, j);
        }
    }
    available.into_iter().take(size).collect()
}

/// AIs pick instantly and randomly from their pool. Personality-aware picks
/// could be a follow-up.
pub(crate) fn ai_draft_system(
    mut draft_state: ResMut<DraftState>,
    game_state: Res<GameState>,
) {
    if game_state.phase != GamePhase::Drafting {
        return;
    }
    if draft_state.picks.is_empty() {
        return; // setup hasn't run yet
    }
    for seat in 1..game_state.players.len() {
        if draft_state.picks[seat].is_some() {
            continue;
        }
        let pool = &draft_state.pools[seat];
        if pool.is_empty() {
            // Player has every buff already — skip silently.
            draft_state.picks[seat] = Some(BuffKind::Mulligan);
            continue;
        }
        let idx = (rand::random::<f32>() * pool.len() as f32) as usize;
        let pick = pool[idx.min(pool.len() - 1)];
        draft_state.picks[seat] = Some(pick);
        info!(
            "AI seat {} ({:?}) picked buff: {}",
            seat,
            game_state.players[seat].personality,
            pick.display_name()
        );
    }
}

/// Spawn the full-screen draft overlay when entering Drafting. Stays up until
/// `apply_picks_system` despawns it (after every seat has chosen). The overlay
/// only shows the human's options; AI picks happen invisibly in the background.
pub(crate) fn draft_screen_system(
    mut commands: Commands,
    game_state: Res<GameState>,
    match_state: Res<MatchState>,
    draft_state: Res<DraftState>,
    screen_q: Query<Entity, With<DraftScreen>>,
    asset_server: Res<AssetServer>,
) {
    if game_state.phase != GamePhase::Drafting {
        return;
    }
    if !screen_q.is_empty() {
        return;
    }
    if draft_state.pools.is_empty() {
        return; // setup hasn't run yet
    }
    let human_pool = draft_state.pools.first().cloned().unwrap_or_default();
    if human_pool.is_empty() {
        return; // nothing to choose — apply_picks will fill it automatically next frame
    }

    let ui_font = asset_server.load("fonts/Rubik-Regular.ttf");
    let pixel_font = asset_server.load("fonts/Silkscreen-Regular.ttf");
    let subtitle = if human_pool.len() >= 5 {
        format!(
            "Round {} \u{00B7} Shed bonus \u{2014} pick 1 of {}",
            match_state.round,
            human_pool.len()
        )
    } else {
        format!("Round {} \u{00B7} click a perk to add it to your run", match_state.round)
    };

    commands
        .spawn((
            DraftScreen,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                background_color: theme::VEIL.into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(
                TextBundle::from_section(
                    "PICK A PERK",
                    TextStyle { font: pixel_font.clone(), font_size: 34.0, color: theme::MAGENTA },
                )
                .with_text_justify(JustifyText::Center),
            );
            parent.spawn(TextBundle {
                text: Text::from_section(
                    subtitle,
                    TextStyle { font: ui_font.clone(), font_size: 14.0, color: theme::MUTED_TEXT },
                ),
                style: Style {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
                ..default()
            });

            for &kind in &human_pool {
                let rarity = theme::buff_rarity(kind);
                let rcolor = rarity.color();
                parent
                    .spawn((
                        DraftOption(kind),
                        ButtonBundle {
                            style: Style {
                                // Responsive: fills most of a narrow portrait
                                // phone but caps at the original desktop width so
                                // the rows don't sprawl on a wide screen.
                                width: Val::Percent(90.0),
                                max_width: Val::Px(460.0),
                                padding: UiRect::all(Val::Px(10.0)),
                                column_gap: Val::Px(12.0),
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            // Near-black solid fill so light perk text stays
                            // legible regardless of the table cards showing through
                            // the veil; the rarity colour becomes the row's border
                            // + accent rather than the whole fill.
                            background_color: ITEM_BG.into(),
                            border_color: rcolor.into(),
                            border_radius: BorderRadius::all(Val::Px(10.0)),
                            ..default()
                        },
                    ))
                    .with_children(|row| {
                        // Icon chip — a rarity-tinted square holding the glyph in the
                        // rarity colour, echoing the row's border accent.
                        row.spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(50.0),
                                height: Val::Px(50.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            background_color: rcolor.with_alpha(0.18).into(),
                            border_radius: BorderRadius::all(Val::Px(8.0)),
                            ..default()
                        })
                        .with_children(|chip| {
                            chip.spawn(TextBundle::from_section(
                                theme::buff_icon(kind),
                                TextStyle { font: pixel_font.clone(), font_size: 18.0, color: rcolor },
                            ));
                        });

                        // Text column: name + rarity tag, then the blurb.
                        row.spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(3.0),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|col| {
                            let name = if kind.is_consumable() {
                                format!("{}  (consumable)", kind.display_name())
                            } else {
                                kind.display_name().to_string()
                            };
                            // Light text for contrast on the near-black fill.
                            col.spawn(TextBundle::from_section(
                                name,
                                TextStyle { font: ui_font.clone(), font_size: 19.0, color: Color::WHITE },
                            ));
                            col.spawn(TextBundle::from_section(
                                rarity.label(),
                                TextStyle {
                                    font: pixel_font.clone(),
                                    font_size: 14.0,
                                    color: rcolor,
                                },
                            ));
                            col.spawn(TextBundle::from_section(
                                kind.description(),
                                TextStyle {
                                    font: ui_font.clone(),
                                    font_size: 13.0,
                                    color: Color::srgba(1.0, 1.0, 1.0, 0.78),
                                },
                            ));
                        });
                    });
            }
        });
}

pub(crate) fn handle_draft_click(
    mut draft_state: ResMut<DraftState>,
    game_state: Res<GameState>,
    mut interaction_q: Query<
        (&Interaction, &DraftOption, &mut BackgroundColor),
        Changed<Interaction>,
    >,
) {
    if game_state.phase != GamePhase::Drafting {
        return;
    }
    let already_picked = draft_state
        .picks
        .first()
        .copied()
        .flatten()
        .is_some();
    for (interaction, option, mut bg) in &mut interaction_q {
        match *interaction {
            Interaction::Pressed => {
                if !already_picked && !draft_state.picks.is_empty() {
                    draft_state.picks[0] = Some(option.0);
                    info!("You picked buff: {}", option.0.display_name());
                }
                *bg = theme::LIME.into();
            }
            Interaction::Hovered => *bg = theme::lighten(ITEM_BG, 0.12).into(),
            Interaction::None => *bg = ITEM_BG.into(),
        }
    }
}

/// Finalize: push each seat's pick into Player.modifiers, snapshot to MatchState
/// so the buff survives the round-end teardown, despawn the overlay, and flip
/// the phase to Playing so the rest of the game wakes up.
pub(crate) fn apply_picks_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    mut match_state: ResMut<MatchState>,
    mut draft_state: ResMut<DraftState>,
    screen_q: Query<Entity, With<DraftScreen>>,
) {
    if game_state.phase != GamePhase::Drafting {
        return;
    }
    if !draft_state.all_picked() {
        return;
    }
    for seat in 0..game_state.players.len() {
        if let Some(kind) = draft_state.picks[seat] {
            // Skip duplicates so consumables can't be double-charged. The draft
            // pool already filters owned kinds, but the AI fallback path can
            // still pick a duplicate if the catalogue is exhausted.
            if !game_state.players[seat].has_buff(kind) {
                game_state.players[seat].modifiers.push(ActiveBuff {
                    kind,
                    used_this_round: false,
                });
            }
        }
    }
    // Snapshot for next-round carry-over.
    for seat in 0..game_state.players.len() {
        if seat < match_state.persistent_modifiers.len() {
            match_state.persistent_modifiers[seat] = game_state.players[seat].modifiers.clone();
        }
    }
    for e in screen_q.iter() {
        commands.entity(e).despawn_recursive();
    }
    draft_state.pools.clear();
    draft_state.picks.clear();
    game_state.phase = GamePhase::Playing;
    info!("Draft complete — entering Playing");
}
