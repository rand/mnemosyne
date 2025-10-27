"""
Low-Latency Context Monitor for Multi-Agent Orchestration.

Provides 10ms polling of context utilization with automatic preservation
triggers at 75% threshold. Integrated with PyCoordinator for shared state.

Performance Targets:
- Polling interval: 10ms
- Per-poll overhead: <1ms
- Memory footprint: <10MB
- Preservation trigger: <50ms
"""

import asyncio
import time
from dataclasses import dataclass
from typing import Optional, Callable, Dict, Any
from enum import Enum


class ContextState(Enum):
    """Context utilization states."""
    SAFE = "safe"           # < 50% utilization
    MODERATE = "moderate"   # 50-75% utilization
    HIGH = "high"           # 75-90% utilization
    CRITICAL = "critical"   # > 90% utilization


@dataclass
class ContextMetrics:
    """Context utilization metrics."""
    utilization: float          # 0.0 - 1.0
    total_tokens: int
    used_tokens: int
    available_tokens: int
    state: ContextState
    timestamp: float
    agent_count: int
    skill_count: int
    file_count: int


class LowLatencyContextMonitor:
    """
    High-frequency context monitor with <1ms per-poll overhead.

    Uses PyCoordinator for shared state management, enabling all agents
    to read context metrics without blocking.

    Features:
    - 10ms polling interval (100 checks/second)
    - Automatic preservation at 75% threshold
    - Configurable callbacks for state transitions
    - Zero-copy metric sharing via PyCoordinator
    """

    def __init__(
        self,
        coordinator,
        polling_interval: float = 0.01,  # 10ms
        preservation_threshold: float = 0.75,
        critical_threshold: float = 0.90
    ):
        """
        Initialize context monitor.

        Args:
            coordinator: PyCoordinator instance for shared state
            polling_interval: Polling interval in seconds (default: 10ms)
            preservation_threshold: Trigger preservation at this utilization
            critical_threshold: Trigger emergency compaction at this utilization
        """
        self.coordinator = coordinator
        self.polling_interval = polling_interval
        self.preservation_threshold = preservation_threshold
        self.critical_threshold = critical_threshold

        # State
        self._running = False
        self._monitor_task: Optional[asyncio.Task] = None
        self._last_metrics: Optional[ContextMetrics] = None
        self._last_state = ContextState.SAFE

        # Callbacks
        self._preservation_callback: Optional[Callable] = None
        self._critical_callback: Optional[Callable] = None
        self._state_change_callback: Optional[Callable] = None

        # Statistics
        self._poll_count = 0
        self._total_poll_time = 0.0
        self._preservation_count = 0
        self._critical_count = 0

    def set_preservation_callback(self, callback: Callable[[ContextMetrics], None]):
        """Set callback for preservation threshold trigger."""
        self._preservation_callback = callback

    def set_critical_callback(self, callback: Callable[[ContextMetrics], None]):
        """Set callback for critical threshold trigger."""
        self._critical_callback = callback

    def set_state_change_callback(self, callback: Callable[[ContextState, ContextState], None]):
        """Set callback for context state changes."""
        self._state_change_callback = callback

    async def start(self):
        """Start monitoring loop."""
        if self._running:
            return

        self._running = True
        self._monitor_task = asyncio.create_task(self._monitor_loop())

    async def stop(self):
        """Stop monitoring loop."""
        if not self._running:
            return

        self._running = False
        if self._monitor_task:
            self._monitor_task.cancel()
            try:
                await self._monitor_task
            except asyncio.CancelledError:
                pass
        self._monitor_task = None

    async def _monitor_loop(self):
        """Main monitoring loop - runs at 10ms intervals."""
        while self._running:
            start_time = time.perf_counter()

            try:
                # Poll current metrics (<1ms target)
                metrics = await self._poll_metrics()

                # Update coordinator with current utilization
                self.coordinator.update_context_utilization(metrics.utilization)

                # Check for state transitions
                await self._check_thresholds(metrics)

                # Track statistics
                self._last_metrics = metrics
                self._poll_count += 1
                poll_time = time.perf_counter() - start_time
                self._total_poll_time += poll_time

                # Emit warning if poll took too long
                if poll_time > 0.001:  # >1ms
                    self.coordinator.set_metric("context_monitor_slow_poll", poll_time)

            except Exception as e:
                # Log error but don't crash monitor
                self.coordinator.set_metric("context_monitor_error", 1.0)
                print(f"Context monitor error: {e}")

            # Sleep until next poll (10ms interval)
            elapsed = time.perf_counter() - start_time
            sleep_time = max(0, self.polling_interval - elapsed)
            await asyncio.sleep(sleep_time)

    async def _poll_metrics(self) -> ContextMetrics:
        """
        Poll current context metrics.

        This should complete in <1ms. Uses coordinator metrics
        and lightweight token counting.

        Returns:
            ContextMetrics with current utilization data
        """
        # Get shared metrics from coordinator
        agent_states = self.coordinator.get_all_agent_states()
        agent_count = len([s for s in agent_states.values() if s == "running"])

        # Get context utilization from coordinator
        utilization = self.coordinator.get_context_utilization()

        # Calculate token estimates (fast approximation)
        # Full context window: ~200k tokens (Claude 3.5 Sonnet)
        total_tokens = 200000
        used_tokens = int(utilization * total_tokens)
        available_tokens = total_tokens - used_tokens

        # Determine state
        if utilization < 0.5:
            state = ContextState.SAFE
        elif utilization < 0.75:
            state = ContextState.MODERATE
        elif utilization < 0.90:
            state = ContextState.HIGH
        else:
            state = ContextState.CRITICAL

        # Get skill/file counts from coordinator metrics
        skill_count = int(self.coordinator.get_metric("skill_count") or 0)
        file_count = int(self.coordinator.get_metric("file_count") or 0)

        return ContextMetrics(
            utilization=utilization,
            total_tokens=total_tokens,
            used_tokens=used_tokens,
            available_tokens=available_tokens,
            state=state,
            timestamp=time.time(),
            agent_count=agent_count,
            skill_count=skill_count,
            file_count=file_count
        )

    async def _check_thresholds(self, metrics: ContextMetrics):
        """Check and trigger threshold callbacks."""
        # State change callback
        if metrics.state != self._last_state:
            if self._state_change_callback:
                await self._maybe_async(
                    self._state_change_callback(self._last_state, metrics.state)
                )
            self._last_state = metrics.state

        # Preservation threshold (75%)
        if metrics.utilization >= self.preservation_threshold:
            if metrics.utilization < self.critical_threshold:
                # Only trigger preservation, not critical
                if self._preservation_callback:
                    self._preservation_count += 1
                    await self._maybe_async(self._preservation_callback(metrics))

        # Critical threshold (90%)
        if metrics.utilization >= self.critical_threshold:
            if self._critical_callback:
                self._critical_count += 1
                await self._maybe_async(self._critical_callback(metrics))

    async def _maybe_async(self, result):
        """Handle both sync and async callbacks."""
        if asyncio.iscoroutine(result):
            await result

    def get_current_metrics(self) -> Optional[ContextMetrics]:
        """Get most recent metrics (non-blocking)."""
        return self._last_metrics

    def get_statistics(self) -> Dict[str, Any]:
        """Get monitoring statistics."""
        avg_poll_time = (
            self._total_poll_time / self._poll_count if self._poll_count > 0 else 0
        )

        return {
            "poll_count": self._poll_count,
            "avg_poll_time_ms": avg_poll_time * 1000,
            "total_poll_time_s": self._total_poll_time,
            "preservation_count": self._preservation_count,
            "critical_count": self._critical_count,
            "current_utilization": (
                self._last_metrics.utilization if self._last_metrics else 0.0
            ),
            "current_state": (
                self._last_metrics.state.value if self._last_metrics else "unknown"
            ),
            "running": self._running
        }

    def is_preservation_needed(self) -> bool:
        """Check if preservation is currently needed."""
        if not self._last_metrics:
            return False
        return self._last_metrics.utilization >= self.preservation_threshold

    def is_critical(self) -> bool:
        """Check if context is in critical state."""
        if not self._last_metrics:
            return False
        return self._last_metrics.utilization >= self.critical_threshold

    def get_available_budget(self) -> int:
        """Get available token budget."""
        if not self._last_metrics:
            return 0
        return self._last_metrics.available_tokens
