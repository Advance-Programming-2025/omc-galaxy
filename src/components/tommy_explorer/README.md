# Tommy Explorer

`tommy_explorer` is a fully autonomous, Actor-Model based agent designed for the `omc-galaxy` game. It traverses dynamic planet topologies, harvests elemental resources, and optimizes complex crafting chains. Built with strong concurrency guarantees, rigorous message buffering, and a strict State Machine, this explorer guarantees non-blocking operation while maximizing resource efficiency.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [File Structure](#2-file-structure)
3. [Operating Modes](#3-operating-modes)
4. [State Machine](#4-state-machine)
5. [Topology & Pathfinding](#5-topology--pathfinding)
6. [AI Engine & Workflow](#6-ai-engine--workflow)
7. [The Bag (Resource Inventory)](#7-the-bag-resource-inventory)
8. [Message Buffering System](#8-message-buffering-system)
9. [Graceful Surrender](#9-graceful-surrender)
10. [Tests](#10-tests)

---

## 1. Architecture Overview

The explorer is an **actor** running in its own thread. It communicates with the Orchestrator and Planets via `crossbeam_channel` bidirectional channels.

The explorer is implemented as a **strict state machine** that processes messages from both channels. To prevent cross-thread race conditions, an action can only be triggered if the state is legally compatible. When a message does not match the current state, it is pushed into a buffer and processed later to guarantee eventual consistency across the cluster.

### AI Goal

The core objective of the AI is to **craft the most complex possible resource** (such as `AIPartner`) while navigating a partially obscured map.


## 2. File Structure

The project is structured to strictly decouple decision-making logic from network packet handling and spatial memory:

```
omc-galaxy/
└── src/
    └── components/
        ├── tommy_explorer/
        │   ├── mod.rs          # Explorer module entry point
        │   ├── core.rs         # Main thread event loop and message reception
        │   ├── state.rs        # Finite State Machine (ExplorerState)
        │   ├── actions.rs      # Micro-action queues (ActionQueue, MoveQueue)
        │   ├── bag.rs          # Inventory and resource management
        │   ├── topology.rs     # Spatial memory, graph mapping, and pathfinding algorithms
        │   └── test.rs         # Comprehensive unit and integration test suite
        └── handlers/
            ├── orchestrator.rs # Handles inbound/outbound messages with the Orchestrator
            └── planet.rs       # Handles direct interactions and hardware requests with Planets
```

| File | Description |
|---|---|
| `core.rs` | The brain and network handler of the explorer. Contains the main event loop, handles the `crossbeam_channel` communication, and triggers AI routines. |
| `state.rs` | Defines the `ExplorerState` enum and controls the strict state-machine rules. Dictates whether a message can be processed instantly or must be buffered. |
| `actions.rs` | Implements the `ActionQueue` (managing micro-tasks) and `MoveQueue` (handling spatial pathing and navigation lists). |
| `bag.rs` | An inventory wrapper (`Bag`) handling internal logic for storing/converting resources. |
| `topology.rs` | The spatial memory containing the `TopologyManager` and graph algorithms for mapping and calculating BFS-based routes. |
| `tests.rs` | A highly comprehensive suite of unit and integration tests mimicking full multi-thread message passing. |
| `orchestrator.rs` | Manages all the orchestrator messages sending back the expected response. |
| `planet.rs` | Manages all the planet messages sending back the expected response. |

---

## 3. Operating Modes

The explorer supports distinct operational modes governed by the Orchestrator:

| Mode | Behaviour |
|---|---|
| **Manual Mode** | The explorer reacts strictly to Orchestrator commands. Activated via `StopExplorerAI`. |
| **AI Mode** | The explorer operates autonomously, triggering its internal loop. Activated via `StartExplorerAI` or `ResetExplorerAI`. |

---

## 4. State Machine

The Explorer is strictly bound by `ExplorerState`.

### States

```rust
enum ExplorerState {
    Idle,
    Traveling,
    WaitingForNeighbours,
    WaitingForSupportedResources,
    WaitingForSupportedCombinations,
    GeneratingResource,
    CombiningResources,
    Killed,
}
```

| State | Description |
|---|---|
| `Idle` | The default state. The Explorer is ready to evaluate its AI loop or process buffered messages. |
| `Traveling` | A blocking state representing transit between planets. Most incoming requests are buffered. |
| `WaitingForNeighbours` | Waiting for the Orchestrator to reply with topology data. |
| `WaitingForSupported*` | Waiting for the local planet to expose its resources or combinations. |
| `Generating / Combining` | The AI is actively engaged communicating with planets to craft resources. |
| `Killed` | A terminal state triggering immediate thread cleanup and memory deallocation. |

---

## 5. Topology & Pathfinding

The navigation relies heavily on the `TopologyManager`. Before targeting specific resources, the AI maps the galaxy.

### Exploration Phase (`find_path_to_nearest_frontier`)

**Full Scan:** The AI queries the Orchestrator and adjacent planets to discover every single node and link in the galaxy.

**Mapping:** Every new coordinate or transition is registered within the `TopologyManager`. The AI does not stop exploring until the entire graph is fully discovered and there are no unknown "frontiers" left.

---

## 6. AI Engine & Workflow

The AI utilizes a **backwards-resolution algorithm** to satisfy its crafting needs through two distinct phases.

### Ultimate Objective & Logic Flow

1. It analyzes its final complex resource goal and recursively decomposes it into intermediate dependencies.
2. It evaluates its current `Bag` and computes the exact delta of missing basic resources.
3. If it is sitting on a planet that provides a missing resource, it generates it (provided it has enough energy).
4. Once all subcomponents are harvested, it attempts to combine them using the planet's facilities.

### Targeting Phase (`find_path_to_resource`)

**Dependency Decomposition:** The AI analyzes its complex resource goal and recursively breaks down its dependency tree until it reaches the required basic resources.

**Optimal Localization:** By querying the complete graph map obtained in Phase 1, the AI calculates the shortest path (using BFS/Dijkstra algorithms) to the specific planets that host the missing basic resources.

**Harvesting & Crafting:** The explorer navigates along the plotted route, draws necessary energy from planetary cells, generates the basic elements, and finally travels to the appropriate nodes to combine them into complex materials.

---

## 7. The Bag (Resource Inventory)

The inventory system (`Bag`) handles strict internal logic for storing and converting `BasicResource` and `ComplexResource` instances. It guarantees type safety and ensures that ingredients are correctly extracted and consumed when the AI issues crafting commands during the `CombiningResources` state.

---

## 8. Message Buffering System

The game heavily utilizes asynchronous `crossbeam_channel`s, meaning the Orchestrator or a Planet can send a message while the Explorer is busy (e.g., `Traveling` or `GeneratingResource`).

To prevent data loss and protocol desynchronization, `tommy_explorer` implements a **dual-queue Message Buffer**:

- `buffer_orchestrator_msg`
- `buffer_planet_msg`

### Buffer Flow

1. When a message is received, it is checked against `state.matches_orchestrator_msg()`.
2. If the current state forbids handling the message immediately, the message is pushed to the back of the queue (**FIFO**).
3. As soon as the Explorer transitions back to `Idle`, it drains and processes the buffer before executing its next autonomous AI action.

> **Note:** This strict buffering guarantees eventual consistency across the cluster, ensuring no commands or survey responses are ever dropped.

---

## 9. Graceful Surrender

If the AI reaches a hard deadlock — such as running completely out of unvisited frontiers while still lacking required resources — it enters a deliberate **Surrender Routine**.

It gracefully accepts its fate by setting `self.accept_death = true`.

---

## 10. Tests

The `tests.rs` file contains a highly comprehensive suite of unit and integration tests mimicking full multi-thread message passing, race conditions, and integration faults.

To run the tests without facing concurrent initialization conflicts over the global generator, always force a single test thread runner:

```bash
cargo nextest run --no-fail-fast tommy_explorer::test
```