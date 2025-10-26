"""
Real-Time Monitoring Dashboard for Multi-Agent Orchestration.

Provides live visualization of:
- Context utilization with state indicators
- Active agents and their states
- Parallel task execution progress
- Quality gate results
- Performance metrics

Uses Rich library for terminal UI with live updates.
"""

import asyncio
from typing import Dict, Any, Optional
from datetime import datetime
from rich.console import Console
from rich.live import Live
from rich.layout import Layout
from rich.panel import Panel
from rich.table import Table
from rich.progress import Progress, SpinnerColumn, BarColumn, TextColumn, TimeElapsedColumn
from rich.text import Text
from rich.style import Style

from .context_monitor import ContextState


class OrchestrationDashboard:
    """
    Real-time monitoring dashboard for orchestration engine.

    Features:
    - Live context utilization graph
    - Agent status table
    - Task progress bars
    - Performance metrics
    - Quality gate indicators
    """

    def __init__(self, engine):
        """
        Initialize dashboard.

        Args:
            engine: OrchestrationEngine instance to monitor
        """
        self.engine = engine
        self.console = Console()
        self.layout = None
        self._live = None
        self._refresh_rate = 0.1  # 100ms refresh
        self._running = False

    async def start(self):
        """Start dashboard display."""
        self._running = True

        # Create layout
        self.layout = self._create_layout()

        # Start live display
        with Live(self.layout, console=self.console, refresh_per_second=10) as live:
            self._live = live

            while self._running:
                # Update layout
                self._update_layout()

                # Refresh
                await asyncio.sleep(self._refresh_rate)

    def stop(self):
        """Stop dashboard display."""
        self._running = False

    def _create_layout(self) -> Layout:
        """Create dashboard layout."""
        layout = Layout()

        # Split into header, body, footer
        layout.split_column(
            Layout(name="header", size=3),
            Layout(name="body"),
            Layout(name="footer", size=5)
        )

        # Split body into left and right
        layout["body"].split_row(
            Layout(name="left", ratio=2),
            Layout(name="right", ratio=1)
        )

        # Split left into top and bottom
        layout["left"].split_column(
            Layout(name="context", ratio=1),
            Layout(name="agents", ratio=2)
        )

        return layout

    def _update_layout(self):
        """Update all dashboard panels."""
        if not self.layout:
            return

        # Get engine status
        status = self.engine.get_status()

        # Update header
        self.layout["header"].update(
            Panel(
                self._render_header(),
                title="Mnemosyne Multi-Agent Orchestration",
                border_style="bold blue"
            )
        )

        # Update context panel
        self.layout["context"].update(
            Panel(
                self._render_context(status.get("context", {})),
                title="Context Utilization",
                border_style=self._get_context_border_style(status.get("context", {}))
            )
        )

        # Update agents panel
        self.layout["agents"].update(
            Panel(
                self._render_agents(status.get("agents", {})),
                title="Agent Status",
                border_style="cyan"
            )
        )

        # Update parallel executor panel
        self.layout["right"].update(
            Panel(
                self._render_parallel_executor(status.get("parallel_executor", {})),
                title="Parallel Execution",
                border_style="magenta"
            )
        )

        # Update footer
        self.layout["footer"].update(
            Panel(
                self._render_footer(),
                border_style="dim"
            )
        )

    def _render_header(self) -> Text:
        """Render header with timestamp."""
        now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        text = Text()
        text.append("Status: ", style="bold")
        text.append("ACTIVE", style="bold green")
        text.append(f"  |  Time: {now}", style="dim")
        return text

    def _render_context(self, context: Dict[str, Any]) -> Table:
        """Render context utilization."""
        table = Table(show_header=False, box=None, padding=(0, 1))

        # Utilization
        utilization = context.get("utilization", 0.0)
        state = context.get("state", "unknown")
        agent_count = context.get("agent_count", 0)

        # Utilization bar
        bar_width = 40
        filled = int(utilization * bar_width)
        empty = bar_width - filled

        bar = "█" * filled + "░" * empty
        bar_style = self._get_utilization_style(utilization)

        table.add_row(
            "Utilization:",
            Text(bar, style=bar_style),
            f"{utilization:.1%}"
        )

        # State
        state_style = self._get_state_style(state)
        table.add_row(
            "State:",
            Text(state.upper(), style=state_style),
            f"{agent_count} agents"
        )

        return table

    def _render_agents(self, agents: Dict[str, Any]) -> Table:
        """Render agent status table."""
        table = Table(show_header=True, header_style="bold")
        table.add_column("Agent", style="cyan")
        table.add_column("Phase", style="yellow")
        table.add_column("Status", justify="center")
        table.add_column("Metrics", justify="right")

        # Orchestrator
        orch = agents.get("orchestrator", {})
        table.add_row(
            "Orchestrator",
            orch.get("phase", "idle"),
            self._render_status_indicator(orch),
            f"{orch.get('checkpoints', 0)} checkpoints"
        )

        # Optimizer
        opt = agents.get("optimizer", {})
        table.add_row(
            "Optimizer",
            "-",
            self._render_status_indicator(opt),
            f"{opt.get('loaded_skills', 0)} skills loaded"
        )

        # Reviewer
        rev = agents.get("reviewer", {})
        pass_rate = rev.get("current_pass_rate", 0.0)
        table.add_row(
            "Reviewer",
            "-",
            self._render_status_indicator(rev),
            f"{pass_rate:.0%} pass rate"
        )

        # Executor
        exe = agents.get("executor", {})
        table.add_row(
            "Executor",
            exe.get("phase", "idle"),
            self._render_status_indicator(exe),
            f"{exe.get('completed_tasks', 0)} tasks done"
        )

        return table

    def _render_parallel_executor(self, executor: Dict[str, Any]) -> Table:
        """Render parallel executor status."""
        table = Table(show_header=False, box=None, padding=(0, 1))

        running = executor.get("running_tasks", 0)
        completed = executor.get("completed_tasks", 0)
        failed = executor.get("failed_tasks", 0)
        total = executor.get("total_tasks", 0)
        max_concurrent = executor.get("max_concurrent", 4)

        # Running tasks bar
        if max_concurrent > 0:
            bar_width = 20
            filled = int((running / max_concurrent) * bar_width)
            empty = bar_width - filled
            bar = "█" * filled + "░" * empty
        else:
            bar = "░" * 20

        table.add_row("Running:", bar, f"{running}/{max_concurrent}")
        table.add_row()
        table.add_row("Completed:", "✓", str(completed))
        table.add_row("Failed:", "✗" if failed > 0 else "-", str(failed))

        if total > 0:
            progress = (completed + failed) / total
            table.add_row()
            table.add_row("Progress:", "", f"{progress:.0%}")

        return table

    def _render_footer(self) -> Table:
        """Render footer with help text."""
        table = Table(show_header=False, box=None, padding=(0, 2))

        table.add_row(
            Text("Press ", style="dim"),
            Text("Ctrl+C", style="bold red"),
            Text(" to stop", style="dim"),
            Text("  |  ", style="dim"),
            Text("Refresh: 100ms", style="dim")
        )

        return table

    def _render_status_indicator(self, agent_data: Dict) -> Text:
        """Render status indicator for agent."""
        # For now, just show a green dot
        # In full implementation, would check actual agent state
        return Text("●", style="green")

    def _get_context_border_style(self, context: Dict) -> str:
        """Get border style based on context state."""
        state = context.get("state", "unknown")
        return {
            "safe": "green",
            "moderate": "yellow",
            "high": "yellow bold",
            "critical": "red bold"
        }.get(state, "white")

    def _get_utilization_style(self, utilization: float) -> str:
        """Get style based on utilization level."""
        if utilization < 0.5:
            return "green"
        elif utilization < 0.75:
            return "yellow"
        elif utilization < 0.90:
            return "yellow bold"
        else:
            return "red bold"

    def _get_state_style(self, state: str) -> str:
        """Get style based on context state."""
        return {
            "safe": "green",
            "moderate": "yellow",
            "high": "yellow bold",
            "critical": "red bold"
        }.get(state, "white")


# Standalone dashboard runner
async def run_dashboard(engine):
    """
    Run dashboard as standalone task.

    Args:
        engine: OrchestrationEngine to monitor

    Usage:
        dashboard_task = asyncio.create_task(run_dashboard(engine))
        # ... run orchestration ...
        dashboard_task.cancel()
    """
    dashboard = OrchestrationDashboard(engine)

    try:
        await dashboard.start()
    except asyncio.CancelledError:
        dashboard.stop()
        print("\nDashboard stopped")


# CLI entry point for standalone dashboard
if __name__ == "__main__":
    import sys

    print("Mnemosyne Orchestration Dashboard")
    print("=" * 40)
    print()
    print("Usage:")
    print("  python -m orchestration.dashboard")
    print()
    print("Note: Dashboard requires an active OrchestrationEngine.")
    print("      Use: mnemosyne orchestrate --dashboard --plan '<prompt>'")
    sys.exit(0)
