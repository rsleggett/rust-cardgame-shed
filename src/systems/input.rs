//! Mouse + keyboard input: hover, click staging, double-click play, the
//! invalid-card-clicked event + its handler, and the Enter/Escape confirm
//! key bindings.

use bevy::prelude::*;

use crate::components::card::Card;
use crate::components::game::{BuffKind, GamePhase, GameState};
use crate::rendering::card_constants::{CARD_HEIGHT, CARD_WIDTH};
use crate::rendering::card_renderer::{Layout, ReducedMotion};
use crate::rules::can_play_card;
use crate::systems::play::{animate_pickup, pickup_cards_in_play, play_selection};

/// Fires when the player clicks a card that cannot legally be played right now.
#[derive(Event)]
pub(crate) struct InvalidCardClicked(pub(crate) Entity);

/// Counts down while the pile-status text is highlighted in orange (invalid-play feedback).
#[derive(Resource, Default)]
pub(crate) struct InvalidFeedbackTimer(pub(crate) f32);

/// Which card the human is hovering (or None). Written by update_hovered_card,
/// read by layout_cards and update_card_visuals to raise/tint the card.
#[derive(Resource, Default)]
pub struct HoveredCard(pub Option<Entity>);

/// Tracks the most recently clicked card and how long ago. A second click on
/// the same card within `DOUBLE_CLICK_WINDOW` skips staging and plays it
/// directly. Cleared once the window lapses to avoid stale match-ups.
#[derive(Resource, Default)]
pub(crate) struct LastClick {
    pub(crate) entity: Option<Entity>,
    pub(crate) age: f32,
}

pub(crate) const DOUBLE_CLICK_WINDOW: f32 = 0.3;

/// The primary pointer-press position this frame in window/viewport space: a
/// left mouse-button press, or failing that the first newly-pressed touch.
/// Touch positions share the same coordinate space as `cursor_position`, so the
/// world-space hit-testing downstream is identical for mouse and touch — this is
/// what makes the table playable on a phone.
pub(crate) fn primary_pointer_press(
    mouse: &ButtonInput<MouseButton>,
    touches: &Touches,
    window: &Window,
) -> Option<Vec2> {
    if mouse.just_pressed(MouseButton::Left) {
        window.cursor_position()
    } else {
        touches.iter_just_pressed().next().map(|t| t.position())
    }
}

pub(crate) fn update_hovered_card(
    mut hovered: ResMut<HoveredCard>,
    game_state: Res<GameState>,
    transforms: Query<&GlobalTransform>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
) {
    // Only track hover on the human player's turn
    if game_state.phase != GamePhase::Playing || game_state.current_player != 0 {
        hovered.0 = None;
        return;
    }

    let (camera, camera_transform) = camera_q.single();
    let window = windows.single();

    let Some(cursor_pos) = window.cursor_position() else {
        hovered.0 = None;
        return;
    };
    let Some(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        hovered.0 = None;
        return;
    };

    let player = &game_state.players[0];
    let draw_pile_not_empty = !game_state.draw_pile.is_empty();
    let source: &[Entity] = if draw_pile_not_empty || !player.hand.is_empty() {
        &player.hand
    } else if !player.face_up_cards.is_empty() {
        &player.face_up_cards
    } else {
        &player.face_down_cards
    };

    // Find the topmost (last in slice = highest z) card under the cursor
    let mut found = None;
    for &entity in source {
        if let Ok(t) = transforms.get(entity) {
            let pos = t.translation().truncate();
            if world_pos.x >= pos.x - CARD_WIDTH / 2.0
                && world_pos.x <= pos.x + CARD_WIDTH / 2.0
                && world_pos.y >= pos.y - CARD_HEIGHT / 2.0
                && world_pos.y <= pos.y + CARD_HEIGHT / 2.0
            {
                found = Some(entity); // later entries overwrite — highest z wins
            }
        }
    }
    hovered.0 = found;
}

pub(crate) fn tick_last_click(mut last_click: ResMut<LastClick>, time: Res<Time>) {
    if last_click.entity.is_some() {
        last_click.age += time.delta_seconds();
        if last_click.age > DOUBLE_CLICK_WINDOW * 2.0 {
            last_click.entity = None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_mouse_input(
    mut commands: Commands,
    windows: Query<&Window>,
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
    transforms: Query<&GlobalTransform>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    layout: Res<Layout>,
    reduced: Res<ReducedMotion>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mut invalid_ev: EventWriter<InvalidCardClicked>,
    mut last_click: ResMut<LastClick>,
) {
    if game_state.phase != GamePhase::Playing {
        return;
    }

    let (camera, camera_transform) = camera_q.single();
    let window = windows.single();

    let Some(pointer) = primary_pointer_press(&mouse_button_input, &touches, window) else { return; };
    let Some(world_position) = camera.viewport_to_world_2d(camera_transform, pointer) else { return; };

    // Click play pile to pick up when required. While pickup is pending, no
    // other card interaction is allowed — the player must pick up before
    // flipping a face-down or staging anything else.
    if game_state.needs_to_pickup && game_state.current_player == 0 {
        let pile_x = layout.play_pile_x();
        let pile_scale = layout.pile_scale();
        let hx = CARD_WIDTH / 2.0 * pile_scale + 12.0;
        let hy = CARD_HEIGHT / 2.0 * pile_scale + 12.0;
        let in_pile = world_position.x >= pile_x - hx
            && world_position.x <= pile_x + hx
            && world_position.y >= -hy
            && world_position.y <= hy;
        if in_pile {
            let current_player_index = game_state.current_player;
            let picked = pickup_cards_in_play(&mut game_state, current_player_index);
            animate_pickup(&mut commands, &transforms, &layout, &game_state, reduced.0, current_player_index, &picked);
            game_state.needs_to_pickup = false;
            game_state.advance_to_next_active();
        }
        return;
    }

    // Only stage cards on the human player's turn
    if game_state.current_player != 0 { return; }

    let current_player_index = game_state.current_player;
    let player = &game_state.players[current_player_index];
    let draw_pile_not_empty = !game_state.draw_pile.is_empty();
    let hand_not_empty = !player.hand.is_empty();
    let face_up_not_empty = !player.face_up_cards.is_empty();

    let playing_from_face_down = !draw_pile_not_empty && !hand_not_empty && !face_up_not_empty;
    let playing_from_face_up = !draw_pile_not_empty && !hand_not_empty && face_up_not_empty;

    let mut cards_to_check: Vec<Entity> = Vec::new();
    if playing_from_face_down {
        cards_to_check.extend(player.face_down_cards.iter().copied());
    } else if playing_from_face_up {
        cards_to_check.extend(player.face_up_cards.iter().copied());
    } else {
        cards_to_check.extend(player.hand.iter().copied());
    }

    // Find the topmost card under the cursor (same strategy as update_hovered_card:
    // iterate all candidates and let later/higher-z entries overwrite earlier ones).
    let mut hit_entity: Option<Entity> = None;
    for &card_entity in &cards_to_check {
        if let Ok(transform) = transforms.get(card_entity) {
            let card_pos = transform.translation().truncate();
            if world_position.x >= card_pos.x - CARD_WIDTH / 2.0
                && world_position.x <= card_pos.x + CARD_WIDTH / 2.0
                && world_position.y >= card_pos.y - CARD_HEIGHT / 2.0
                && world_position.y <= card_pos.y + CARD_HEIGHT / 2.0
            {
                hit_entity = Some(card_entity); // later (higher-z) entries overwrite
            }
        }
    }

    let Some(card_entity) = hit_entity else { return; };

    // Face-down: blind immediate play. Validity is resolved by play_selection,
    // which routes an invalid play to a pickup instead of flashing red — the
    // player can't know what the card is before flipping it.
    if playing_from_face_down {
        play_selection(&mut commands, &mut game_state, &cards, &transforms, &layout, &[card_entity]);
        // Don't seed last_click — face-down already plays on a single click.
        last_click.entity = None;
        return;
    }

    let Ok(card) = cards.get(card_entity) else { return; };

    // Face-up endgame: any face-up may be staged. An invalid attempt at confirm
    // time pushes the staged cards onto the pile and triggers pickup, matching
    // physical Shed rules (you commit to a face-up; if it bricks, you eat it
    // along with the pile).
    if !playing_from_face_up {
        let has_counter7 = game_state.players[0].has_buff(BuffKind::Counter7);
        if !can_play_card(
            card,
            game_state.effective_rank,
            game_state.seven_active,
            game_state.any_card_playable,
            has_counter7,
        ) {
            invalid_ev.send(InvalidCardClicked(card_entity));
            // An invalid click still resets the double-click tracker so the
            // next click on a different card starts a fresh window.
            last_click.entity = None;
            return;
        }
    }

    // Double-click on the same card within the window plays it directly,
    // bypassing the staging step. Works for hand plays (re-validated above)
    // and face-up endgame plays (play_selection handles invalid via pickup).
    let is_double_click =
        last_click.entity == Some(card_entity) && last_click.age < DOUBLE_CLICK_WINDOW;
    if is_double_click {
        game_state.selected_cards.clear();
        play_selection(&mut commands, &mut game_state, &cards, &transforms, &layout, &[card_entity]);
        last_click.entity = None;
        return;
    }
    last_click.entity = Some(card_entity);
    last_click.age = 0.0;

    let card_rank = card.rank;
    if game_state.selected_cards.contains(&card_entity) {
        // Deselect
        game_state.selected_cards.retain(|&e| e != card_entity);
    } else {
        let sel_rank = game_state.selected_cards.first()
            .and_then(|&e| cards.get(e).ok())
            .map(|c| c.rank);
        if sel_rank.is_none() || sel_rank == Some(card_rank) {
            if game_state.selected_cards.len() < 4 {
                game_state.selected_cards.push(card_entity);
            }
        } else {
            // Different rank — start fresh selection
            game_state.selected_cards = vec![card_entity];
        }
    }
}

/// Applies red flash to the clicked card and highlights the pile status text.
pub(crate) fn handle_invalid_card_event(
    mut events: EventReader<InvalidCardClicked>,
    mut cards: Query<&mut Card>,
    mut feedback: ResMut<InvalidFeedbackTimer>,
) {
    for InvalidCardClicked(entity) in events.read() {
        if let Ok(mut card) = cards.get_mut(*entity) {
            card.invalid_timer = 0.5;
        }
        feedback.0 = 2.0;
    }
}

/// Confirms (Enter) or cancels (Escape) the staged multi-card play.
pub(crate) fn confirm_play_system(
    mut commands: Commands,
    mut game_state: ResMut<GameState>,
    cards: Query<&Card>,
    transforms: Query<&GlobalTransform>,
    layout: Res<Layout>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if game_state.phase != GamePhase::Playing || game_state.current_player != 0 { return; }

    if keyboard.just_pressed(KeyCode::Escape) {
        game_state.selected_cards.clear();
        return;
    }
    if !keyboard.just_pressed(KeyCode::Enter) { return; }
    if game_state.selected_cards.is_empty() { return; }

    let selection = std::mem::take(&mut game_state.selected_cards);
    play_selection(&mut commands, &mut game_state, &cards, &transforms, &layout, &selection);
}
