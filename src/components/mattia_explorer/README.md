# Mattia Explorer

A Rust actor-based explorer component that navigates a planetary network, collects resources, and can operate either under manual orchestrator control or fully autonomously via a utility-based AI engine.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [File Structure](#2-file-structure)
3. [Getting Started](#3-getting-started)
4. [Operating Modes](#4-operating-modes)
5. [State Machine](#5-state-machine)
6. [Message Flow](#6-message-flow)
7. [The Bag (Resource Inventory)](#7-the-bag-resource-inventory)
8. [Planet Information & Classification](#8-planet-information--classification)
9. [AI System](#9-ai-system)
10. [AI Configuration Parameters](#10-ai-configuration-parameters)
11. [Buffering System](#11-buffering-system)
12. [Panic Safety Reference](#12-panic-safety-reference)

---

## 1. Architecture Overview

The explorer is an **actor** running in its own thread. It communicates with two external entities via `crossbeam_channel` bidirectional channels.

The explorer is implemented as a **state machine** that simultaneously processes messages from both channels. When a message does not match the current state, it is pushed into a buffer and processed later.

### AI Goal

The primary objective of the AI is **to survive as long as possible in the galaxy**. To do so, it continuously monitors the safety of the current planet (based on energy cells, recharge rate, escape routes, and rocket availability) and autonomously decides when to produce resources, explore new planets, or flee to safer locations.

---

## 2. File Structure

```
src/components/mattia_explorer/
├── mod.rs                  # Explorer struct, main loop, constructor
├── explorer_ai.rs          # Utility-based AI engine
├── ai_params.rs            # All tunable AI parameters
├── handlers.rs             # Handler for each message type
├── helpers.rs              # Utility functions (gather_info_from_planet)
├── states.rs               # State enum and matching functions
├── bag.rs                  # Resource inventory
├── buffers.rs              # Buffered message management
├── resource_management.rs  # ToGeneric trait for resource conversion
├── planet_info.rs          # Topological data per planet
├── tests.rs                # Test suite
└── test_topology_files/
    └── t0.txt              # Topology file used in tests
```

---

## 3. Getting Started

### Construction

Create an explorer with `Explorer::new(...)`, providing communication channels with the orchestrator and the initial planet. The constructor:

- Accepts `crossbeam_channel` pairs for both the orchestrator and the planet
- Initialises the topology map with only the starting planet
- Starts in `Idle` state with an empty `Bag`
- Sets the internal time counter to `1`
- Starts in **manual mode** (`manual_mode = true`)

To customise AI behaviour at construction time, use `Explorer::with_params(AiParams { ... })` instead. This lets you tune every aspect of the AI decision engine — safety thresholds, decay factors, weights, hysteresis margins, and more — without recompiling. See [Section 10](#10-ai-configuration-parameters) for the full list of parameters.

If the explorer also needs to **produce resources** (in addition to surviving), you can set the `ResourceNeeds` fields inside `AiParams` at construction time. Each field is a `f32` in `[0.0, 1.0]` representing how urgently that resource is needed; needs propagate down the crafting tree automatically, so setting a need for a complex resource (e.g. `AIPartner`) will also raise the need for its ingredients.

### Running

Call `explorer.run()` to start the main loop. This method blocks until the explorer receives a `KillExplorer` message from the orchestrator, at which point it returns `Ok(())`.


---

## 4. Operating Modes

The explorer has two operating modes, toggled via orchestrator messages:

| Mode | `manual_mode` | Behaviour |
|------|---------------|-----------|
| **Manual** | `true` (default) | The explorer only reacts to orchestrator commands |
| **AI** | `false` | The AI autonomously decides the next action when all of the following are true: `manual_mode == false`, `state == Idle`, no orchestrator messages are available, no planet messages are available, and no buffered messages are processable. |

### Switching Modes

Send the following messages from the orchestrator:

- `StartExplorerAI` → activates AI mode, explorer becomes autonomous
- `StopExplorerAI` → returns to manual mode
- `ResetExplorerAI` → resets topology and AI state, re-activates AI mode
- `KillExplorer` → terminates the explorer thread (accepted in **any** state)

---

## 5. State Machine

### States

```rust
enum ExplorerState {
    Idle,                                          // ready to receive or execute actions
    WaitingForNeighbours,                          // waiting for neighbour list from orchestrator
    Traveling,                                     // travelling to another planet
    GeneratingResource { orchestrator_response },  // generating a basic resource
    CombiningResources { orchestrator_response },  // combining a complex resource
    Surveying {
        resources,      // waiting for basic resource list
        combinations,   // waiting for combination list
        energy_cells,   // waiting for energy cell data
        orch_resource,  // orchestrator requested resource info
        orch_combination, // orchestrator requested combination info
    },
    Killed,                                        // explorer has been terminated
}
```

The `orchestrator_response` flag in `GeneratingResource` and `CombiningResources` signals whether the action was initiated by the orchestrator (requiring a reply) or by the AI (no reply needed).

### Message Acceptance Rules

- `Idle` accepts all messages from both channels.
- `NeighborsResponse` is only accepted in `WaitingForNeighbours`.
- `MoveToPlanet` is only accepted in `Traveling`.
- `GenerateResourceResponse` is only accepted in `GeneratingResource`.
- `CombineResourceResponse` is only accepted in `CombiningResources`.
- Survey responses are only accepted when the corresponding `Surveying` flag is `true`.
- `KillExplorer` and `StopExplorerAI` are **always** accepted, regardless of state.

---

## 6. Message Flow

### Orchestrator → Explorer → Planet → Explorer → Orchestrator

Example with `SupportedResourceRequest`:

```
1. Orchestrator sends SupportedResourceRequest to Explorer
2. Explorer checks if the info is already in its topology
   a. YES → sends SupportedResourceResult back to Orchestrator immediately
   b. NO  →
      - Sets state to Surveying (orch_resource = true)
      - Forwards SupportedResourceRequest to Planet
      - When the Planet respond saves the info in topology
      - Forwards SupportedResourceResult to Orchestrator
      - Returns to Idle
```

### AI-Initiated Flow

```
1. Main loop, default branch: AI active, state = Idle
2. ai_core_function() is called
3. calc_utility() scores all possible actions
4. find_best_action() picks the best one
5. Explorer sends the appropriate message (to planet or orchestrator)
6. Explorer changes state (e.g. Traveling, GeneratingResource, …)
7. Response is handled in the main loop
```

### Travel Protocol

```
1. Explorer (or AI) decides to travel
2. Explorer sends TravelToPlanetRequest to Orchestrator
   (with explorer_id, current_planet_id, dst_planet_id)
3. Explorer sets state to Traveling
4. Orchestrator coordinates IncomingExplorer / OutgoingExplorer with planets
5. Orchestrator sends MoveToPlanet to Explorer
   (with sender_to_new_planet and planet_id)
6. move_to_planet() updates channels, topology, and state
7. If not in manual mode: initiates a survey of the new planet
```

> **Planet disconnection**: if the planet channel disconnects (e.g. the planet dies), the explorer does **not** terminate. It logs an error, disables the planet channel to avoid busy-waiting, and continues waiting for `KillExplorer` from the orchestrator.

### Termination

```
1. Orchestrator sends KillExplorer (accepted in ANY state)
2. kill_explorer() sets state to Killed, sends KillExplorerResult
3. Main loop detects Killed state → return Ok(()) → thread exits
```

---

## 7. The Bag (Resource Inventory)

The bag stores typed vectors of resources, keeping type safety throughout:

### Basic Resources

`Oxygen`, `Hydrogen`, `Carbon`, `Silicon`

### Complex Resources (Crafting Recipes)

| Complex Resource | Ingredient 1 | Ingredient 2 |
|-----------------|--------------|--------------|
| Diamond | Carbon | Carbon |
| Water | Hydrogen | Oxygen |
| Life | Water | Carbon |
| Robot | Silicon | Life |
| Dolphin | Water | Life |
| AIPartner | Robot | Diamond |

### Key Methods

| Method | Description |
|--------|-------------|
| `insert(res)` | Inserts a generic resource into the appropriate typed vector |
| `take_resource(ty)` | Extracts (pops) one resource of the given type; returns `Option` |
| `contains(ty)` | Returns `true` if at least one resource of the given type exists |
| `count(ty)` | Returns the count of a given resource type |
| `can_craft(complex_type)` | Returns `(can_craft, type_r1, has_r1, type_r2, has_r2)` |
| `to_resource_types()` | Returns a `Vec<ResourceType>` snapshot without transferring ownership |

The `make_*_request()` methods (e.g. `make_water_request()`) verify `can_craft()`, extract the required ingredients, and build a `ComplexResourceRequest` ready to send to the planet.

---

## 8. Planet Information & Classification

### Planet Classes

| Class | Can have Rocket | Max Energy Cells |
|-------|----------------|-----------------|
| A | Yes | 5 |
| B | No | 1 |
| C | Yes | 1 |
| D | No | 5 |

Planet type is **inferred** from the resources it supports:

| Condition | Inferred Type |
|-----------|--------------|
| `complex_resources > 1` | C |
| `complex_resources == 1` | B (likely) |
| `complex_resources == 0 && basic_resources > 1` | D |
| `complex_resources == 0 && basic_resources <= 1` | A (likely) |

> **Note on heuristic classification**: types marked *(likely)* are not guaranteed. Classification is heuristic because resource information alone cannot always uniquely identify the underlying planet class (e.g. a planet with exactly one complex resource could be either B or C). The inferred type is used by the AI as a best guess, particularly to determine whether the planet can have a rocket.

### PlanetInfo Fields

Each known planet is tracked via a `PlanetInfo` struct:

| Field | Type | Description |
|-------|------|-------------|
| `basic_resources` | `Option<HashSet<BasicResourceType>>` | Producible basic resources |
| `complex_resources` | `Option<HashSet<ComplexResourceType>>` | Combinable complex resources |
| `neighbors` | `Option<HashSet<ID>>` | Known neighbouring planet IDs |
| `energy_cells` | `Option<u32>` | Last observed energy cell count |
| `charge_rate` | `Option<f32>` | Estimated recharge rate (EMA with α=0.3) |
| `timestamp_neighbors` | `u64` | Tick of the last neighbour update |
| `timestamp_energy` | `u64` | Tick of the last energy update |
| `safety_score` | `Option<f32>` | Safety score in `[0.0, 1.0]` |
| `inferred_planet_type` | `Option<PlanetClassType>` | Deduced planet class |

The charge rate is updated using an **exponential moving average (EMA)**:
`new_rate = 0.3 * instant_rate + 0.7 * old_rate`

---

## 9. AI System

The AI is **utility-based**: each cycle it scores every possible action and executes the one with the highest score.

### Possible Actions

```rust
enum AIActionType {
    Produce(BasicResourceType),   // generate a basic resource on the current planet
    Combine(ComplexResourceType), // craft a complex resource on the current planet
    MoveTo(ID),                   // travel to a neighbouring planet
    SurveyNeighbors,              // request neighbour info from orchestrator
    SurveyEnergy,                 // request energy info from planet
    Wait,                         // do nothing this tick
    RunAway,                      // flee to the safest reachable planet
}
```

### Safety Score

Each planet receives a safety score in `[0.0, 1.0]` composed of three weighted components:

| Component | Weight | Description |
|-----------|--------|-------------|
| Sustainability | 0.15 | Based on charge rate (1.0 active, 0.7 slow, 0.5 none) |
| Physical safety | 0.70 | Based on predicted energy cells; multiplied by `rocket` factor (`1.0` if the planet can have a rocket, `0.3` otherwise) |
| Escape factor | 0.15 | Based on number of known neighbours (0→0.2, 1→0.5, 2→0.8, 3+→1.0) |

**Formula**: `(sustainability×0.15 + physical_safety×rocket×0.70 + escape×0.15) × noise`

Safety thresholds:
- `safety_score < SAFETY_CRITICAL (0.3)` → immediate evacuation triggered
- `safety_score < SAFETY_WARNING (0.6)` → explorer starts looking for safer planets

### Resource Need Propagation

Resource needs propagate **down the crafting tree** with a configurable `propagation_factor` (default `0.8`). Example: a need for `AIPartner` propagates to `Robot` and `Diamond`, which in turn propagate to `Silicon`, `Life`, `Carbon`, and so on.

```
Level 4: AIPartner
Level 3: Robot, Dolphin
Level 2: Life, Diamond
Level 1: Water
Level 0: Carbon, Oxygen, Hydrogen, Silicon  ← basic resources
```

### Information Decay

The AI discounts stale information using exponential decay:
`reliability = e^(-λ × Δt)` where `Δt` is the number of ticks since the data was collected. This means older data is trusted less when making decisions.

### AI Core Loop

```
1. First visit to this planet?
   ├─ YES → request neighbours → return
   └─ NO  → continue

2. Missing basic or complex resource info?
   ├─ YES → survey planet → return
   └─ NO  → continue

3. calc_utility()    — score all actions
4. find_best_action() — pick the winner
5. Execute:
   ├─ RunAway        → TravelToPlanetRequest to the safest neighbour
   ├─ MoveTo(id)     → TravelToPlanetRequest to id
   ├─ SurveyNeighbors → NeighborsRequest
   ├─ SurveyEnergy   → gather_info_from_planet (energy only)
   ├─ Produce(res)   → GenerateResourceRequest to planet
   ├─ Combine(res)   → extract ingredients from bag, CombineResourceRequest
   └─ Wait           → do nothing
```

---

## 10. AI Configuration Parameters

All AI parameters are grouped in `AiParams` and can be passed at construction time via `Explorer::with_params(AiParams { ... })`. Below are all fields and their defaults.

| Parameter | Default | Description |
|-----------|---------|-------------|
| `randomness_range` | `0.1` | Noise multiplier range applied to utility scores (`[1-val, 1+val]`) |
| `lambda` | `0.005` | Exponential decay factor for information staleness: `e^(-λ·Δt)` |
| `propagation_factor` | `0.8` | Need propagation factor through the crafting tree |
| `safety_critical` | `0.3` | Critical danger threshold — triggers immediate evacuation |
| `safety_warning` | `0.6` | Warning threshold — explorer starts seeking safer planets |
| `energy_cells_defense_threshold` | `2` | Minimum energy cells to consider a planet "defended" |
| `max_energy_info_age` | `150` | Ticks after which energy info is considered stale |
| `action_hysteresis_margin` | `0.07` | Minimum score advantage required to switch actions |
| `min_active_charge_rate` | `0.05` | Minimum charge rate to consider a planet "actively recharging" |
| `max_prediction_horizon` | `100` | Maximum future ticks for energy predictions |
| `perfect_info_max_time` | `10` | Ticks within which information is considered perfectly accurate |
| `safety_min_diff` | `0.07` | Minimum safety improvement required to justify fleeing |
| `wait_base` | `0.08` | Base utility score for the Wait action |
| `wait_bonus` | `0.1` | Additional utility for Wait when on a safe, recharging planet |
| `safety_weight_sustainability` | `0.15` | Weight of the sustainability component in the safety score |
| `safety_weight_physical` | `0.70` | Weight of the physical safety component in the safety score |
| `safety_weight_escape` | `0.15` | Weight of the escape factor in the safety score |
| `charge_rate_alpha` | `0.3` | EMA alpha for smoothing charge rate estimates |

---

## 11. Buffering System

When a message arrives but does not match the current state (e.g. `BagContentRequest` arrives while the explorer is `Traveling`), it is pushed onto the appropriate buffer queue. Each main loop cycle — when both channels are empty — the buffer manager checks whether the front message in each queue now matches the current state. If it does, the message is dequeued and processed normally. If not, it stays in the queue.

- Orchestrator messages → `buffer_orchestrator_msg: VecDeque<OrchestratorToExplorer>`
- Planet messages → `buffer_planet_msg: VecDeque<PlanetToExplorer>`

> `KillExplorer` is accepted in any state and therefore can never end up in the buffer in practice.

---

## 12. Panic Safety Reference

The codebase uses `unwrap()` in several places, all of which are guarded:

| Location | Guard mechanism |
|----------|----------------|
| `bag.rs` — `make_*_request()` methods | Always preceded by `can_craft()` check; single-threaded, no TOCTOU risk |
| `handlers.rs` — topology `get_mut().unwrap()` | Element is `insert()`-ed in the line immediately before |
| `buffers.rs` — `front().unwrap()` / `pop_front().unwrap()` | Protected by `!is_empty()` check before access |
| `explorer_ai.rs` — `unwrap_or()` calls | Provide safe fallback values; cannot panic |
| `mod.rs` — time counter | Uses `wrapping_add(1)` instead of `+`; wraps at `u64::MAX` without panic |

Logically impossible situations are handled with warning logs rather than panics, so the explorer continues running.