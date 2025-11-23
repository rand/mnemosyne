# Network Panel Specification

## Goal
Implement a real-time network panel in the dashboard (`src/bin/dash`) to visualize the P2P network state.

## Architecture
- **Backend**:
  - `AgentEvent::NetworkStateUpdate`: New event type carrying network stats.
  - `Daemon`: Periodically queries `router` and broadcasts `NetworkStateUpdate`.
- **Frontend**:
  - `NetworkPanel`: New TUI component.
  - `App`: Integration of the new panel.

## Changes

### 1. `src/orchestration/events.rs`
- Add `NetworkStateUpdate` variant to `AgentEvent`.
- Fields:
  - `connected_peers: usize`
  - `known_nodes: Vec<String>`
- Update `importance()` and `summary()` methods.

### 2. `src/daemon/orchestration.rs`
- In `run_engine`:
  - Start a periodic task (every 5s).
  - Query `network.router.list_agents()`.
  - Broadcast `AgentEvent::NetworkStateUpdate`.

### 3. `src/bin/dash/panels/network.rs` (New)
- Struct `NetworkPanel`.
- State:
  - `connected_peers: usize`
  - `known_nodes: Vec<String>`
  - `activity_log: Vec<String>`
- Logic:
  - `update(event)`: Handle `NetworkStateUpdate`.
  - `render()`: Draw the UI.

### 4. `src/bin/dash/main.rs`
- Register `NetworkPanel` in `App`.
- Add keybind `'n'` or `'4'` to toggle it.
- Route `NetworkStateUpdate` events to it.
- Update `render_panels` to include it.
