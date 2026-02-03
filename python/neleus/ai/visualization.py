"""
Agent Action Visualization

Provides visualization for agent actions and demo results:
- Terminal-based charts
- HTML report generation
- Action timeline visualization
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional
import html


@dataclass
class ActionVisualization:
    """A visualized agent action."""
    timestamp: datetime
    action_type: str
    icon: str
    title: str
    content: str
    color: str
    duration_ms: Optional[float] = None


class TerminalVisualizer:
    """
    ASCII-art visualization for terminal output.
    
    Creates visual timelines and charts in the terminal.
    """
    
    # Unicode box drawing characters
    CHARS = {
        "h_line": "─",
        "v_line": "│",
        "tl_corner": "┌",
        "tr_corner": "┐",
        "bl_corner": "└",
        "br_corner": "┘",
        "t_junction": "┬",
        "b_junction": "┴",
        "l_junction": "├",
        "r_junction": "┤",
        "cross": "┼",
        "arrow_right": "►",
        "arrow_down": "▼",
        "bullet": "●",
        "check": "✓",
        "cross_mark": "✗",
    }
    
    # ANSI colors
    COLORS = {
        "reset": "\033[0m",
        "bold": "\033[1m",
        "dim": "\033[2m",
        "blue": "\033[94m",
        "green": "\033[92m",
        "yellow": "\033[93m",
        "red": "\033[91m",
        "cyan": "\033[96m",
        "magenta": "\033[95m",
        "white": "\033[97m",
    }
    
    def __init__(self, use_colors: bool = True, width: int = 70):
        self.use_colors = use_colors
        self.width = width
    
    def _c(self, text: str, color: str) -> str:
        """Apply color to text."""
        if self.use_colors:
            return f"{self.COLORS.get(color, '')}{text}{self.COLORS['reset']}"
        return text
    
    def draw_box(self, title: str, content: List[str], color: str = "white") -> str:
        """Draw a box around content."""
        lines = []
        
        # Top border with title
        title_line = f"{self.CHARS['tl_corner']}{self.CHARS['h_line']} {title} "
        title_line += self.CHARS['h_line'] * (self.width - len(title) - 5) + self.CHARS['tr_corner']
        lines.append(self._c(title_line, color))
        
        # Content
        for line in content:
            padded = f"{self.CHARS['v_line']} {line}"
            padded = padded.ljust(self.width - 1) + self.CHARS['v_line']
            lines.append(padded)
        
        # Bottom border
        bottom = self.CHARS['bl_corner'] + self.CHARS['h_line'] * (self.width - 2) + self.CHARS['br_corner']
        lines.append(self._c(bottom, color))
        
        return "\n".join(lines)
    
    def draw_timeline(self, actions: List[ActionVisualization]) -> str:
        """Draw a timeline of actions."""
        lines = []
        
        lines.append(self._c("\n  AGENT ACTION TIMELINE", "bold"))
        lines.append(self._c("  " + "═" * (self.width - 4), "dim"))
        
        for i, action in enumerate(actions):
            is_last = i == len(actions) - 1
            connector = self.CHARS['bl_corner'] if is_last else self.CHARS['l_junction']
            
            # Time
            time_str = action.timestamp.strftime("%H:%M:%S")
            
            # Icon and type
            icon_color = {
                "thinking": "blue",
                "tool_call": "green",
                "decision": "yellow",
                "observation": "cyan",
                "error": "red",
            }.get(action.action_type, "white")
            
            icon = {
                "thinking": "🧠",
                "tool_call": "🔧",
                "decision": "📊",
                "observation": "👁",
                "error": "❌",
            }.get(action.action_type, "•")
            
            line = f"  {self.CHARS['v_line']}  [{time_str}] {icon} {action.title}"
            if action.duration_ms:
                line += self._c(f" ({action.duration_ms:.1f}ms)", "dim")
            lines.append(line)
            
            # Content preview (truncated)
            if action.content:
                content_preview = action.content[:50] + "..." if len(action.content) > 50 else action.content
                lines.append(f"  {self.CHARS['v_line']}              {self._c(content_preview, 'dim')}")
            
            # Connector
            if not is_last:
                lines.append(f"  {self.CHARS['v_line']}")
        
        lines.append(self._c("  " + "═" * (self.width - 4), "dim"))
        
        return "\n".join(lines)
    
    def draw_metrics_chart(
        self,
        title: str,
        metrics: Dict[str, float],
        max_bar_width: int = 30,
    ) -> str:
        """Draw a horizontal bar chart for metrics."""
        lines = []
        
        lines.append(self._c(f"\n  {title}", "bold"))
        lines.append("  " + "─" * (self.width - 4))
        
        # Find max value for scaling
        max_val = max(abs(v) for v in metrics.values()) if metrics else 1
        
        for label, value in metrics.items():
            # Calculate bar width
            bar_width = int((abs(value) / max_val) * max_bar_width)
            
            # Determine color based on metric type
            if "pnl" in label.lower() or "return" in label.lower() or "profit" in label.lower():
                color = "green" if value >= 0 else "red"
            elif "risk" in label.lower() or "drawdown" in label.lower() or "loss" in label.lower():
                color = "red" if value > 10 else "yellow"
            else:
                color = "cyan"
            
            # Draw bar
            bar = "█" * bar_width + "░" * (max_bar_width - bar_width)
            
            # Format value
            if isinstance(value, float):
                val_str = f"{value:>8.2f}"
            else:
                val_str = f"{value:>8}"
            
            line = f"  {label:>20}: {self._c(bar, color)} {val_str}"
            lines.append(line)
        
        lines.append("  " + "─" * (self.width - 4))
        
        return "\n".join(lines)
    
    def draw_summary_card(
        self,
        agent_name: str,
        duration_s: float,
        tool_calls: int,
        decisions: int,
        success_rate: float,
    ) -> str:
        """Draw a summary card for an agent session."""
        content = [
            f"Agent: {self._c(agent_name, 'cyan')}",
            f"Duration: {duration_s:.1f}s",
            "",
            f"  {self._c('Tool Calls:', 'bold')} {tool_calls}",
            f"  {self._c('Decisions:', 'bold')} {decisions}",
            f"  {self._c('Success Rate:', 'bold')} {success_rate:.1f}%",
        ]
        
        return self.draw_box("SESSION SUMMARY", content, "magenta")


class HTMLReportGenerator:
    """
    Generate HTML reports from agent actions.
    
    Creates beautiful, interactive HTML pages showing:
    - Action timeline
    - Tool execution results
    - Performance metrics
    """
    
    HTML_TEMPLATE = """
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Neleus Agent Report - {agent_name}</title>
    <style>
        :root {{
            --bg-primary: #0d1117;
            --bg-secondary: #161b22;
            --bg-tertiary: #21262d;
            --text-primary: #e6edf3;
            --text-secondary: #8b949e;
            --accent-blue: #58a6ff;
            --accent-green: #3fb950;
            --accent-yellow: #d29922;
            --accent-red: #f85149;
            --accent-purple: #a371f7;
        }}
        
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
            background-color: var(--bg-primary);
            color: var(--text-primary);
            line-height: 1.6;
        }}
        
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem;
        }}
        
        header {{
            text-align: center;
            padding: 2rem 0;
            border-bottom: 1px solid var(--bg-tertiary);
            margin-bottom: 2rem;
        }}
        
        header h1 {{
            font-size: 2.5rem;
            background: linear-gradient(135deg, var(--accent-blue), var(--accent-purple));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            margin-bottom: 0.5rem;
        }}
        
        header .subtitle {{
            color: var(--text-secondary);
        }}
        
        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 1rem;
            margin-bottom: 2rem;
        }}
        
        .stat-card {{
            background: var(--bg-secondary);
            border-radius: 8px;
            padding: 1.5rem;
            border: 1px solid var(--bg-tertiary);
        }}
        
        .stat-card .label {{
            color: var(--text-secondary);
            font-size: 0.875rem;
            text-transform: uppercase;
            margin-bottom: 0.5rem;
        }}
        
        .stat-card .value {{
            font-size: 2rem;
            font-weight: 600;
        }}
        
        .stat-card .value.positive {{ color: var(--accent-green); }}
        .stat-card .value.negative {{ color: var(--accent-red); }}
        
        .timeline {{
            position: relative;
            padding-left: 2rem;
        }}
        
        .timeline::before {{
            content: '';
            position: absolute;
            left: 0.5rem;
            top: 0;
            bottom: 0;
            width: 2px;
            background: var(--bg-tertiary);
        }}
        
        .timeline-item {{
            position: relative;
            padding: 1rem;
            margin-bottom: 1rem;
            background: var(--bg-secondary);
            border-radius: 8px;
            border: 1px solid var(--bg-tertiary);
        }}
        
        .timeline-item::before {{
            content: '';
            position: absolute;
            left: -1.75rem;
            top: 1.5rem;
            width: 12px;
            height: 12px;
            border-radius: 50%;
            background: var(--accent-blue);
        }}
        
        .timeline-item.tool_call::before {{ background: var(--accent-green); }}
        .timeline-item.decision::before {{ background: var(--accent-yellow); }}
        .timeline-item.observation::before {{ background: var(--accent-blue); }}
        .timeline-item.thinking::before {{ background: var(--accent-purple); }}
        
        .timeline-item .header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 0.5rem;
        }}
        
        .timeline-item .title {{
            font-weight: 600;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }}
        
        .timeline-item .time {{
            color: var(--text-secondary);
            font-size: 0.875rem;
        }}
        
        .timeline-item .content {{
            color: var(--text-secondary);
            font-size: 0.9rem;
        }}
        
        .timeline-item .content pre {{
            background: var(--bg-tertiary);
            padding: 0.75rem;
            border-radius: 4px;
            overflow-x: auto;
            margin-top: 0.5rem;
        }}
        
        .badge {{
            display: inline-block;
            padding: 0.25rem 0.5rem;
            border-radius: 4px;
            font-size: 0.75rem;
            font-weight: 500;
        }}
        
        .badge.success {{ background: rgba(63, 185, 80, 0.2); color: var(--accent-green); }}
        .badge.error {{ background: rgba(248, 81, 73, 0.2); color: var(--accent-red); }}
        
        .section-title {{
            font-size: 1.25rem;
            margin: 2rem 0 1rem;
            padding-bottom: 0.5rem;
            border-bottom: 1px solid var(--bg-tertiary);
        }}
        
        footer {{
            text-align: center;
            padding: 2rem 0;
            color: var(--text-secondary);
            font-size: 0.875rem;
        }}
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>🤖 {agent_name}</h1>
            <p class="subtitle">AI Trading Agent Session Report</p>
            <p class="subtitle">{timestamp}</p>
        </header>
        
        <div class="stats-grid">
            <div class="stat-card">
                <div class="label">Duration</div>
                <div class="value">{duration}s</div>
            </div>
            <div class="stat-card">
                <div class="label">Tool Calls</div>
                <div class="value">{tool_calls}</div>
            </div>
            <div class="stat-card">
                <div class="label">Decisions</div>
                <div class="value">{decisions}</div>
            </div>
            <div class="stat-card">
                <div class="label">Success Rate</div>
                <div class="value {success_class}">{success_rate}%</div>
            </div>
        </div>
        
        <h2 class="section-title">📋 Action Timeline</h2>
        
        <div class="timeline">
            {timeline_items}
        </div>
        
        <footer>
            <p>Generated by Neleus AI Agent Framework</p>
            <p>🚀 Make Your Agent Trade Smarter</p>
        </footer>
    </div>
</body>
</html>
"""
    
    TIMELINE_ITEM_TEMPLATE = """
        <div class="timeline-item {action_type}">
            <div class="header">
                <span class="title">
                    {icon} {title}
                    {badge}
                </span>
                <span class="time">{time}</span>
            </div>
            <div class="content">
                {content}
            </div>
        </div>
"""
    
    def __init__(self, output_dir: Optional[Path] = None):
        self.output_dir = output_dir or Path("reports")
        self.output_dir.mkdir(parents=True, exist_ok=True)
    
    def generate_report(
        self,
        agent_name: str,
        actions: List[Dict[str, Any]],
        summary: Dict[str, Any],
    ) -> Path:
        """Generate an HTML report from agent actions."""
        
        # Generate timeline items
        timeline_html = ""
        for action in actions:
            icon = {
                "thinking": "🧠",
                "tool_call": "🔧",
                "decision": "📊",
                "observation": "👁",
            }.get(action.get("action_type", ""), "•")
            
            title = action.get("tool_name", action.get("action_type", "Unknown"))
            
            # Badge for tool calls
            badge = ""
            if action.get("action_type") == "tool_call":
                if action.get("success", True):
                    badge = '<span class="badge success">Success</span>'
                else:
                    badge = '<span class="badge error">Failed</span>'
            
            # Content
            content = ""
            if action.get("reasoning"):
                content = f"<p>{html.escape(action['reasoning'][:500])}</p>"
            if action.get("output_data"):
                output_str = json.dumps(action["output_data"], indent=2)[:1000]
                content += f"<pre>{html.escape(output_str)}</pre>"
            if action.get("error"):
                content += f"<p style='color: var(--accent-red);'>Error: {html.escape(action['error'])}</p>"
            
            timeline_html += self.TIMELINE_ITEM_TEMPLATE.format(
                action_type=action.get("action_type", ""),
                icon=icon,
                title=title,
                badge=badge,
                time=action.get("timestamp", "")[:19],
                content=content or "<p>No details</p>",
            )
        
        # Generate full HTML
        success_rate = summary.get("successful_tools", 0) / max(1, summary.get("tool_calls", 1)) * 100
        
        html_content = self.HTML_TEMPLATE.format(
            agent_name=agent_name,
            timestamp=datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
            duration=f"{summary.get('total_duration_s', 0):.1f}",
            tool_calls=summary.get("tool_calls", 0),
            decisions=summary.get("decisions", 0),
            success_rate=f"{success_rate:.1f}",
            success_class="positive" if success_rate >= 80 else "negative" if success_rate < 50 else "",
            timeline_items=timeline_html,
        )
        
        # Save file
        report_file = self.output_dir / f"agent_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.html"
        report_file.write_text(html_content)
        
        return report_file


def create_demo_visualization(
    log_file: Path,
    output_format: str = "terminal",
) -> Optional[Path]:
    """
    Create visualization from a log file.
    
    Args:
        log_file: Path to agent log JSON file
        output_format: "terminal" for console output, "html" for report
    
    Returns:
        Path to generated report if HTML, None for terminal
    """
    # Load log file
    with open(log_file) as f:
        log_data = json.load(f)
    
    actions = log_data.get("actions", [])
    agent_name = log_data.get("agent_name", "Unknown")
    
    if output_format == "terminal":
        viz = TerminalVisualizer()
        
        # Convert to ActionVisualization
        visualizations = []
        for action in actions:
            visualizations.append(ActionVisualization(
                timestamp=datetime.fromisoformat(action["timestamp"]),
                action_type=action["action_type"],
                icon="",
                title=action.get("tool_name", action["action_type"]),
                content=action.get("reasoning", str(action.get("output_data", ""))[:100]),
                color="white",
                duration_ms=action.get("duration_ms"),
            ))
        
        # Print timeline
        print(viz.draw_timeline(visualizations))
        
        # Print summary
        tool_calls = [a for a in actions if a["action_type"] == "tool_call"]
        success_count = sum(1 for t in tool_calls if t.get("success", True))
        
        print(viz.draw_summary_card(
            agent_name=agent_name,
            duration_s=len(actions) * 2.0,  # Approximate
            tool_calls=len(tool_calls),
            decisions=len([a for a in actions if a["action_type"] == "decision"]),
            success_rate=success_count / max(1, len(tool_calls)) * 100,
        ))
        
        return None
        
    elif output_format == "html":
        generator = HTMLReportGenerator()
        
        summary = {
            "total_duration_s": len(actions) * 2.0,
            "tool_calls": len([a for a in actions if a["action_type"] == "tool_call"]),
            "decisions": len([a for a in actions if a["action_type"] == "decision"]),
            "successful_tools": len([a for a in actions if a["action_type"] == "tool_call" and a.get("success", True)]),
        }
        
        return generator.generate_report(agent_name, actions, summary)


__all__ = [
    "TerminalVisualizer",
    "HTMLReportGenerator",
    "ActionVisualization",
    "create_demo_visualization",
]
