# Phase 3: Time-Series Visualization

## Overview
Add sparklines and mini-charts to dashboard panels to show temporal trends and historical data, enabling users to understand performance patterns at a glance.

## Goals
- Provide visual trend indicators for key metrics
- Enable historical data tracking without external dependencies
- Maintain dashboard responsiveness with minimal overhead
- Keep visualizations compact (fits within panel real estate)

## Scope

### In Scope
1. **Sparkline Widget** - Compact inline time-series graphs
2. **Ring Buffer Storage** - Fixed-size historical data collection (last N data points)
3. **Panel Integration** - Add sparklines to 4 key panels:
   - Memory: stores/recalls per minute trends
   - Context: budget utilization over time
   - Work: progress rate over time
   - Agents: active agent count trend

### Out of Scope
- Full-featured charting (axes, labels, legends)
- Long-term persistence (memory-only for now)
- Interactive zooming/panning
- Multiple series per sparkline (single metric only)

## Requirements

### Functional
- **FR-1**: Sparkline widget renders 7-char wide mini-graphs using Unicode block characters
- **FR-2**: Ring buffer stores last 50 data points per metric (configurable)
- **FR-3**: Data collection happens on each metrics update (every refresh interval)
- **FR-4**: Sparklines show trend direction and relative magnitude
- **FR-5**: Empty/insufficient data shows placeholder (e.g., "─────")

### Non-Functional
- **NFR-1**: Minimal memory overhead (<10KB per sparkline)
- **NFR-2**: No impact on dashboard refresh rate
- **NFR-3**: Graceful degradation if data unavailable
- **NFR-4**: ASCII-only fallback for non-Unicode terminals

## Design

### Data Structure
```rust
/// Ring buffer for time-series data
pub struct TimeSeriesBuffer<T> {
    data: VecDeque<T>,
    capacity: usize,
}

impl<T> TimeSeriesBuffer<T> {
    pub fn push(&mut self, value: T);
    pub fn as_slice(&self) -> &[T];
    pub fn is_empty(&self) -> bool;
}
```

### Sparkline Widget
```rust
/// Compact sparkline visualization
pub struct Sparkline<'a> {
    data: &'a [f32],
    style: Style,
}

impl<'a> Sparkline<'a> {
    pub fn new(data: &'a [f32]) -> Self;
    pub fn render(&self, area: Rect) -> Vec<Line>;
}
```

### Integration Points
- **MemoryPanel**: Add `stores_history` and `recalls_history` buffers
- **ContextPanel**: Add `budget_history` buffer
- **WorkPanel**: Add `progress_rate_history` buffer
- **AgentsPanel**: Add `agent_count_history` buffer

## Implementation Plan

### Phase 3.1: Sparkline Widget
1. Create `src/bin/dash/widgets/sparkline.rs`
2. Implement Unicode block character rendering
3. Add scaling logic (auto-scale to data range)
4. Unit tests for edge cases

### Phase 3.2: Ring Buffer
1. Create `src/bin/dash/time_series.rs`
2. Implement `TimeSeriesBuffer<T>`
3. Add push/get/clear operations
4. Unit tests for boundary conditions

### Phase 3.3: Panel Integration
1. Update each panel struct with history buffers
2. Modify `update()` methods to push to buffers
3. Integrate sparkline rendering in `render()` methods
4. Update panel layouts to accommodate sparklines

### Phase 3.4: Testing & Polish
1. Visual testing with live data
2. Edge case handling (empty, single point, all same value)
3. Performance profiling
4. Documentation updates

## Success Criteria
- [ ] Sparklines render correctly in 4 panels
- [ ] Historical data persists across refreshes (in-memory)
- [ ] No performance degradation
- [ ] Graceful handling of edge cases
- [ ] Visual validation with real workload

## Test Plan

### Unit Tests
- Sparkline rendering with various data patterns
- Ring buffer edge cases (empty, full, overflow)
- Scaling logic correctness

### Integration Tests
- Panel updates populate history buffers
- Sparklines update on each refresh
- Memory usage remains bounded

### Manual Testing
- Visual inspection of trends
- Verify trends match actual metric changes
- Test terminal resizing behavior

## Timeline Estimate
- Phase 3.1: 30-45 min (sparkline widget)
- Phase 3.2: 15-20 min (ring buffer)
- Phase 3.3: 45-60 min (panel integration)
- Phase 3.4: 20-30 min (testing/polish)
- **Total**: ~2-2.5 hours

## Dependencies
- Existing panel infrastructure (Phase 2)
- No external crates needed (use std::collections::VecDeque)

## Risks & Mitigations
- **Risk**: Sparklines hard to read in small spaces
  - **Mitigation**: Use 7-char width minimum, auto-scale for clarity
- **Risk**: Memory growth with many metrics
  - **Mitigation**: Fixed-size ring buffers (50 points max)
- **Risk**: Unicode rendering issues
  - **Mitigation**: ASCII fallback mode

## Future Enhancements (Phase 4+)
- Multiple series per chart
- Color-coded trend indicators (red=down, green=up)
- Configurable history depth
- Persistence to disk for session continuity
