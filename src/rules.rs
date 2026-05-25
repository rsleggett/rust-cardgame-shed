//! Pure Shed rule predicates. No Bevy types — easy to unit-test and reason
//! about in isolation. Callers in `game_plugin.rs` resolve the buff flags and
//! pile-top ranks once, then ask the question here.

use crate::components::card::{Card, Rank};

/// Whether `card` is legal to play given the current pile state and the
/// playing seat's buffs.
///
/// 2, 3, and 10 ignore all restrictions (they're the always-playable specials).
/// After a 2 the pile resets and `any_card_playable` short-circuits everything.
/// After a 7 the next player is capped at ≤ 7 unless they have Counter-7.
/// Otherwise the played rank must meet or beat `effective_rank`.
pub fn can_play_card(
    card: &Card,
    effective_rank: Option<Rank>,
    seven_active: bool,
    any_card_playable: bool,
    has_counter7: bool,
) -> bool {
    // 2, 3, and 10 are always playable
    if matches!(card.rank, Rank::Two | Rank::Three | Rank::Ten) {
        return true;
    }
    if any_card_playable {
        return true;
    }
    if seven_active && !has_counter7 {
        return (card.rank as u8) <= (Rank::Seven as u8);
    }
    if let Some(r) = effective_rank {
        (card.rank as u8) >= (r as u8)
    } else {
        true
    }
}

/// Whether playing `rank` should burn the pile (move it to discard, same
/// player goes again).
///
/// `pile_ranks` is the pile after the new play, ordered oldest-to-newest.
/// Only the last `threshold` entries are inspected (threshold is 4, or 3
/// with Hot Hand); anything older is ignored, so callers may pass either
/// the whole pile or just the top — both are correct.
///
/// Burn triggers:
///   - any Ten
///   - top `threshold` cards all share `rank` (4-of-a-kind, or 3 with Hot Hand)
///   - a Two played by a Wild Twos holder
///   - a King played by a Wild Kings holder
pub fn is_burn(
    rank: Rank,
    pile_ranks: &[Rank],
    hot_hand: bool,
    wild_twos: bool,
    wild_kings: bool,
) -> bool {
    if rank == Rank::Ten {
        return true;
    }
    if rank == Rank::Two && wild_twos {
        return true;
    }
    if rank == Rank::King && wild_kings {
        return true;
    }
    let threshold = if hot_hand { 3 } else { 4 };
    if pile_ranks.len() < threshold {
        return false;
    }
    let start = pile_ranks.len() - threshold;
    pile_ranks[start..].iter().all(|&r| r == rank)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::card::{Card, Suit};

    fn card(rank: Rank) -> Card {
        Card::new(Suit::Hearts, rank)
    }

    // ── can_play_card ─────────────────────────────────────────────────────

    #[test]
    fn always_playable_specials_ignore_state() {
        for r in [Rank::Two, Rank::Three, Rank::Ten] {
            // Even with a strict ≥ King requirement, 2/3/10 still play.
            assert!(can_play_card(&card(r), Some(Rank::King), false, false, false));
            // Even with seven_active capping ≤ 7, 10 still plays.
            assert!(can_play_card(&card(r), Some(Rank::Seven), true, false, false));
        }
    }

    #[test]
    fn any_card_playable_overrides_effective_rank() {
        assert!(can_play_card(
            &card(Rank::Four),
            Some(Rank::King),
            false,
            true,  // any_card_playable
            false,
        ));
    }

    #[test]
    fn seven_active_caps_at_seven() {
        let make = |r| can_play_card(&card(r), Some(Rank::Seven), true, false, false);
        assert!(make(Rank::Four));
        assert!(make(Rank::Seven));
        // 8 is above 7 → blocked under seven_active.
        assert!(!make(Rank::Eight));
        assert!(!make(Rank::King));
        // 2/3/10 still pass per the special-rank early return.
        assert!(make(Rank::Two));
        assert!(make(Rank::Ten));
    }

    #[test]
    fn counter7_ignores_seven_cap() {
        assert!(can_play_card(
            &card(Rank::King),
            Some(Rank::Seven),
            true,  // seven_active
            false,
            true,  // has_counter7
        ));
    }

    #[test]
    fn effective_rank_requires_ge() {
        let make = |r| can_play_card(&card(r), Some(Rank::Nine), false, false, false);
        assert!(!make(Rank::Eight));
        assert!(make(Rank::Nine));
        assert!(make(Rank::King));
    }

    #[test]
    fn no_effective_rank_allows_any() {
        assert!(can_play_card(&card(Rank::Four), None, false, false, false));
    }

    // ── is_burn ───────────────────────────────────────────────────────────

    #[test]
    fn ten_always_burns() {
        assert!(is_burn(Rank::Ten, &[], false, false, false));
        assert!(is_burn(Rank::Ten, &[Rank::Two], false, false, false));
    }

    #[test]
    fn four_of_a_kind_burns_at_default_threshold() {
        let pile = [Rank::Five, Rank::Five, Rank::Five, Rank::Five];
        assert!(is_burn(Rank::Five, &pile, false, false, false));
    }

    #[test]
    fn three_of_a_kind_only_burns_with_hot_hand() {
        let pile = [Rank::Six, Rank::Six, Rank::Six];
        assert!(!is_burn(Rank::Six, &pile, false, false, false));
        assert!(is_burn(Rank::Six, &pile, true, false, false));
    }

    #[test]
    fn mixed_top_does_not_burn() {
        let pile = [Rank::Four, Rank::Five, Rank::Five, Rank::Five];
        // Top 4 includes a Four — no burn even though last 3 match.
        assert!(!is_burn(Rank::Five, &pile, false, false, false));
        // With Hot Hand we only need top 3 → that's three 5s → burns.
        assert!(is_burn(Rank::Five, &pile, true, false, false));
    }

    #[test]
    fn short_pile_does_not_burn_without_specials() {
        let pile = [Rank::Five, Rank::Five];
        assert!(!is_burn(Rank::Five, &pile, false, false, false));
        assert!(!is_burn(Rank::Five, &pile, true, false, false));
    }

    #[test]
    fn wild_twos_burns_a_two() {
        assert!(is_burn(Rank::Two, &[], false, true, false));
        assert!(!is_burn(Rank::Two, &[], false, false, false));
    }

    #[test]
    fn wild_kings_burns_a_king() {
        assert!(is_burn(Rank::King, &[], false, false, true));
        assert!(!is_burn(Rank::King, &[], false, false, false));
        // Wild Twos doesn't make a King burn.
        assert!(!is_burn(Rank::King, &[], false, true, false));
    }
}
