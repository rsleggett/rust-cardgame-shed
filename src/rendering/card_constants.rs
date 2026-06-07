pub const CARD_WIDTH: f32 = 80.0;
pub const CARD_HEIGHT: f32 = 112.0;
pub const CARD_OVERLAP: f32 = 20.0; // Gap between table card columns (positive = spacing)
pub const Z_INDEX_STEP: f32 = 1.0;   // Ensure enough separation between layers
pub const PLAY_PILE_X: f32 = 150.0;  // World x position of the play pile

pub const HAND_FAN_STEP: f32 = 36.0;  // Horizontal px between card centres in hand fan
pub const HAND_FAN_ANGLE: f32 = 5.0;  // Degrees rotation per offset unit from centre
pub const HAND_FAN_ARC: f32 = 4.0;    // Px vertical drop per unit from centre

/// Vertical room reserved below the human hand for the bottom action bar
/// (Play/Done button + consumable mini-cards), so they never overlap the hand
/// regardless of how many cards it holds.
pub const ACTION_BAR_CLEARANCE: f32 = 96.0;