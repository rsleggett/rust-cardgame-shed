# Card Game - Shit Head

A card game implementation using the Bevy game engine and Rust.

## Overview

This project implements a card game (similar to Shit Head) using the Bevy ECS (Entity Component System) game engine. The game features:

- A standard 52-card deck
- Two players with hands of cards
- Card dealing and animation
- Face-up/face-down card states
- Basic game state management

## Project Structure

- `src/components/` - Contains game components and bundles
  - `card.rs` - Card component definitions
  - `card_visual.rs` - Card visual rendering bundle
  - `game.rs` - Game state and player components
- `src/rendering/` - Rendering systems and plugins
  - `card_renderer.rs` - Card rendering plugin
- `src/plugins/` - Game plugins
  - `game_plugin.rs` - Main game plugin
- `src/main.rs` - Application entry point

## How to Run

```bash
cargo run
```

## Controls

- Space: Switch between players (for testing)

## Future Improvements

### ECS Architecture Improvements

1. **Component vs. Resource Separation**
   - Separate `GameState` (game rules, turn order) from entity references
   - Create a dedicated `GameEntities` resource for entity references
   - This improves separation of concerns and makes the code more maintainable

2. **System Responsibility**
   - Create separate systems for game rules and component state management
   - For example, split `update_card_face_up_state` into:
     - A system that determines what should be face up based on game rules
     - A system that updates the visual state based on component changes

3. **Bundle Usage**
   - Create a dedicated system for spawning game entities based on game state
   - Move entity spawning logic out of `GameState::prepare_dealing`
   - This centralizes entity creation and makes it more consistent

4. **Event-Based Communication**
   - Implement events for game actions (e.g., `CardDealtEvent`, `CardFlippedEvent`)
   - Use events to communicate between systems instead of direct component modification
   - This decouples systems and makes the code more maintainable

5. **Query Organization**
   - Create more focused queries that only access the components they need
   - This improves performance and makes the code more readable

6. **Plugin Organization**
   - Split the `GamePlugin` into separate plugins for:
     - Game logic
     - Rendering
     - Input handling
   - This improves separation of concerns and makes the code more modular

7. **Component Design**
   - Split the `Card` component into `CardData` (suit, rank) and `CardState` (is_face_up)
   - This separates immutable data from mutable state

8. **System Ordering**
   - Use explicit system ordering with `.before()` and `.after()`
   - This ensures systems run in the correct order (e.g., game logic before rendering)

### Gameplay Improvements

1. **Game Rules Implementation**
   - Implement the full Shit Head game rules
   - Add card playing mechanics
   - Add win/lose conditions

2. **UI Improvements**
   - Add a proper UI for game status
   - Add buttons for player actions
   - Add animations for card movements

3. **Multiplayer Support**
   - Add network multiplayer support
   - Implement client-server architecture

4. **AI Players**
   - Add AI players with different difficulty levels
   - Implement card playing strategies

5. **Sound Effects**
   - Add sound effects for card dealing, playing, etc.
   - Add background music

## License

This project is licensed under the MIT License - see the LICENSE file for details. 