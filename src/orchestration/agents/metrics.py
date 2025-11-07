"""
Performance metrics tracking for Python agents.

Tracks execution duration, success rates, and resource usage
for production monitoring and optimization.
"""

import time
from typing import Optional, Dict, Any
from dataclasses import dataclass, field
from datetime import datetime


@dataclass
class WorkItemMetrics:
    """Metrics for a single work item execution."""
    work_item_id: str
    agent_id: str
    phase: str
    start_time: float
    end_time: Optional[float] = None
    duration_seconds: Optional[float] = None
    success: bool = False
    error_type: Optional[str] = None

    # Resource usage
    context_tokens: Optional[int] = None
    api_calls: int = 0

    # Review-specific metrics
    review_passed: Optional[bool] = None
    review_confidence: Optional[float] = None
    quality_gates_passed: int = 0
    quality_gates_failed: int = 0

    # Optimization-specific metrics
    skills_loaded: int = 0
    context_budget_used: Optional[int] = None

    def finalize(self, success: bool, error_type: Optional[str] = None):
        """Mark work item as complete and calculate duration."""
        self.end_time = time.time()
        self.duration_seconds = self.end_time - self.start_time
        self.success = success
        self.error_type = error_type

    def to_dict(self) -> Dict[str, Any]:
        """Convert metrics to dictionary for storage."""
        return {
            "work_item_id": self.work_item_id,
            "agent_id": self.agent_id,
            "phase": self.phase,
            "start_time": datetime.fromtimestamp(self.start_time).isoformat(),
            "end_time": datetime.fromtimestamp(self.end_time).isoformat() if self.end_time else None,
            "duration_seconds": self.duration_seconds,
            "success": self.success,
            "error_type": self.error_type,
            "context_tokens": self.context_tokens,
            "api_calls": self.api_calls,
            "review_passed": self.review_passed,
            "review_confidence": self.review_confidence,
            "quality_gates_passed": self.quality_gates_passed,
            "quality_gates_failed": self.quality_gates_failed,
            "skills_loaded": self.skills_loaded,
            "context_budget_used": self.context_budget_used,
        }


@dataclass
class AgentMetrics:
    """Aggregate metrics for an agent."""
    agent_id: str
    total_work_items: int = 0
    successful_work_items: int = 0
    failed_work_items: int = 0
    total_duration_seconds: float = 0.0
    avg_duration_seconds: float = 0.0
    min_duration_seconds: Optional[float] = None
    max_duration_seconds: Optional[float] = None

    # Review-specific aggregates
    total_reviews: int = 0
    reviews_passed: int = 0
    reviews_failed: int = 0
    avg_review_confidence: float = 0.0

    # Optimization-specific aggregates
    total_skills_loaded: int = 0
    avg_skills_per_task: float = 0.0

    def update(self, work_item_metrics: WorkItemMetrics):
        """Update aggregate metrics with new work item."""
        self.total_work_items += 1

        if work_item_metrics.success:
            self.successful_work_items += 1
        else:
            self.failed_work_items += 1

        if work_item_metrics.duration_seconds:
            self.total_duration_seconds += work_item_metrics.duration_seconds
            self.avg_duration_seconds = self.total_duration_seconds / self.total_work_items

            if self.min_duration_seconds is None or work_item_metrics.duration_seconds < self.min_duration_seconds:
                self.min_duration_seconds = work_item_metrics.duration_seconds

            if self.max_duration_seconds is None or work_item_metrics.duration_seconds > self.max_duration_seconds:
                self.max_duration_seconds = work_item_metrics.duration_seconds

        # Review metrics
        if work_item_metrics.review_passed is not None:
            self.total_reviews += 1
            if work_item_metrics.review_passed:
                self.reviews_passed += 1
            else:
                self.reviews_failed += 1

            if work_item_metrics.review_confidence:
                # Running average
                prev_total = self.avg_review_confidence * (self.total_reviews - 1)
                self.avg_review_confidence = (prev_total + work_item_metrics.review_confidence) / self.total_reviews

        # Optimization metrics
        if work_item_metrics.skills_loaded > 0:
            self.total_skills_loaded += work_item_metrics.skills_loaded
            self.avg_skills_per_task = self.total_skills_loaded / self.total_work_items

    def get_success_rate(self) -> float:
        """Calculate success rate as percentage."""
        if self.total_work_items == 0:
            return 0.0
        return (self.successful_work_items / self.total_work_items) * 100.0

    def get_review_pass_rate(self) -> float:
        """Calculate review pass rate as percentage."""
        if self.total_reviews == 0:
            return 0.0
        return (self.reviews_passed / self.total_reviews) * 100.0

    def to_dict(self) -> Dict[str, Any]:
        """Convert metrics to dictionary."""
        return {
            "agent_id": self.agent_id,
            "total_work_items": self.total_work_items,
            "successful_work_items": self.successful_work_items,
            "failed_work_items": self.failed_work_items,
            "success_rate": self.get_success_rate(),
            "total_duration_seconds": self.total_duration_seconds,
            "avg_duration_seconds": self.avg_duration_seconds,
            "min_duration_seconds": self.min_duration_seconds,
            "max_duration_seconds": self.max_duration_seconds,
            "total_reviews": self.total_reviews,
            "reviews_passed": self.reviews_passed,
            "reviews_failed": self.reviews_failed,
            "review_pass_rate": self.get_review_pass_rate(),
            "avg_review_confidence": self.avg_review_confidence,
            "total_skills_loaded": self.total_skills_loaded,
            "avg_skills_per_task": self.avg_skills_per_task,
        }


class MetricsCollector:
    """
    Centralized metrics collection for Python agents.

    Tracks work item executions and maintains aggregate statistics
    per agent for monitoring and optimization.
    """

    def __init__(self):
        self.work_item_metrics: Dict[str, WorkItemMetrics] = {}
        self.agent_metrics: Dict[str, AgentMetrics] = {}

    def start_work_item(
        self,
        work_item_id: str,
        agent_id: str,
        phase: str
    ) -> WorkItemMetrics:
        """
        Start tracking a work item.

        Args:
            work_item_id: Unique work item identifier
            agent_id: Agent processing the work
            phase: Execution phase

        Returns:
            WorkItemMetrics instance for this work item
        """
        metrics = WorkItemMetrics(
            work_item_id=work_item_id,
            agent_id=agent_id,
            phase=phase,
            start_time=time.time()
        )
        self.work_item_metrics[work_item_id] = metrics
        return metrics

    def finish_work_item(
        self,
        work_item_id: str,
        success: bool,
        error_type: Optional[str] = None
    ) -> Optional[WorkItemMetrics]:
        """
        Finish tracking a work item and update aggregates.

        Args:
            work_item_id: Work item identifier
            success: Whether work completed successfully
            error_type: Type of error if failed

        Returns:
            WorkItemMetrics if found, None otherwise
        """
        metrics = self.work_item_metrics.get(work_item_id)
        if not metrics:
            return None

        # Finalize metrics
        metrics.finalize(success, error_type)

        # Update agent aggregates
        agent_id = metrics.agent_id
        if agent_id not in self.agent_metrics:
            self.agent_metrics[agent_id] = AgentMetrics(agent_id=agent_id)

        self.agent_metrics[agent_id].update(metrics)

        return metrics

    def get_work_item_metrics(self, work_item_id: str) -> Optional[WorkItemMetrics]:
        """Get metrics for a specific work item."""
        return self.work_item_metrics.get(work_item_id)

    def get_agent_metrics(self, agent_id: str) -> Optional[AgentMetrics]:
        """Get aggregate metrics for an agent."""
        return self.agent_metrics.get(agent_id)

    def get_all_agent_metrics(self) -> Dict[str, AgentMetrics]:
        """Get metrics for all agents."""
        return dict(self.agent_metrics)

    def clear_work_item_metrics(self, older_than_seconds: Optional[float] = None):
        """
        Clear old work item metrics to prevent memory growth.

        Args:
            older_than_seconds: Clear metrics older than this many seconds.
                              If None, clear all completed work items.
        """
        if older_than_seconds is None:
            # Clear all completed
            self.work_item_metrics = {
                wid: m for wid, m in self.work_item_metrics.items()
                if m.end_time is None
            }
        else:
            # Clear old completed
            cutoff = time.time() - older_than_seconds
            self.work_item_metrics = {
                wid: m for wid, m in self.work_item_metrics.items()
                if m.end_time is None or m.end_time > cutoff
            }


# Global metrics collector instance
_global_metrics_collector: Optional[MetricsCollector] = None


def get_metrics_collector() -> MetricsCollector:
    """Get or create global metrics collector."""
    global _global_metrics_collector
    if _global_metrics_collector is None:
        _global_metrics_collector = MetricsCollector()
    return _global_metrics_collector


def reset_metrics_collector():
    """Reset global metrics collector (for testing)."""
    global _global_metrics_collector
    _global_metrics_collector = MetricsCollector()
