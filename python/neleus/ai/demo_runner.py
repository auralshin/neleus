#!/usr/bin/env python3
"""
Neleus AI Agent Demo Runner

Demonstrates Neleus AI trading agents powered by Ollama.

Two modes:
1. --reasoning : Shows detailed agent reasoning, tool calls, and decision process
2. --charts    : Visual charts with order book, price chart, and metrics dashboard

Usage:
    python -m neleus.ai.demo_runner --reasoning        # Detailed reasoning view
    python -m neleus.ai.demo_runner --charts           # Visual charts view
    python -m neleus.ai.demo_runner --model llama3.2   # Use different model
"""

import argparse
import asyncio
import logging
import sys
from pathlib import Path
from datetime import datetime, timedelta
import json

# Setup logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    handlers=[logging.StreamHandler()],
)
logger = logging.getLogger(__name__)


def check_ollama() -> bool:
    """Check if Ollama is running."""
    try:
        import urllib.request
        req = urllib.request.Request("http://localhost:11434/api/tags")
        with urllib.request.urlopen(req, timeout=2) as resp:
            return resp.status == 200
    except Exception:
        return False


def print_banner():
    """Print welcome banner."""
    banner = """
╔══════════════════════════════════════════════════════════════════════╗
║                                                                      ║
║   ███╗   ██╗███████╗██╗     ███████╗██╗   ██╗███████╗               ║
║   ████╗  ██║██╔════╝██║     ██╔════╝██║   ██║██╔════╝               ║
║   ██╔██╗ ██║█████╗  ██║     █████╗  ██║   ██║███████╗               ║
║   ██║╚██╗██║██╔══╝  ██║     ██╔══╝  ██║   ██║╚════██║               ║
║   ██║ ╚████║███████╗███████╗███████╗╚██████╔╝███████║               ║
║   ╚═╝  ╚═══╝╚══════╝╚══════╝╚══════╝ ╚═════╝ ╚══════╝               ║
║                                                                      ║
║           AI Trading Agent Demo - Powered by Ollama                  ║
║                                                                      ║
║   "Make Your Agent Trade Smarter"                                    ║
║                                                                      ║
╚══════════════════════════════════════════════════════════════════════╝
"""
    print(banner)


async def run_automated_demo(model: str = "llama3.2", base_url: str = "http://localhost:11434"):
    """
    Run an automated demo showcasing Neleus agent capabilities.
    Basic mode - shows prompts and responses.
    """
    from .ollama_demo import OllamaTradingAgent
    
    print("\n" + "=" * 70)
    print("  AUTOMATED DEMO - Showcasing Neleus AI Agent Capabilities")
    print("=" * 70)
    
    agent = OllamaTradingAgent(
        name="DemoAgent",
        model=model,
        base_url=base_url,
        instruments=["BTC", "ETH", "SOL"],
        log_actions=True,
    )
    
    try:
        await agent.start()
        
        # Demo scenarios
        demo_scenarios = [
            {
                "name": "Market Regime Analysis",
                "prompt": "Analyze the current market regime for BTC and ETH. Tell me if we're in a risk-on or risk-off environment.",
            },
            {
                "name": "Volatility Check",
                "prompt": "Check the current volatility for BTC. Is it in a high or low volatility regime? What does this mean for trading?",
            },
            {
                "name": "Strategy Backtest",
                "prompt": "Run a momentum strategy backtest on BTC from 2025-06-01 to 2025-12-31. Analyze the results - is this strategy profitable?",
            },
            {
                "name": "Risk Assessment",
                "prompt": "I want to take a position of 0.5 BTC. Calculate the risk metrics and tell me if this is an appropriate position size given a $100,000 portfolio.",
            },
            {
                "name": "Trading Recommendation",
                "prompt": "Based on all the analysis we've done, should I be trading BTC right now? Provide a clear recommendation with reasoning.",
            },
        ]
        
        for i, scenario in enumerate(demo_scenarios, 1):
            print(f"\n{'─' * 70}")
            print(f"  SCENARIO {i}/{len(demo_scenarios)}: {scenario['name']}")
            print(f"{'─' * 70}")
            print(f"\n  📝 Prompt: {scenario['prompt'][:80]}...")
            print()
            
            response = await agent.think_and_act(scenario["prompt"])
            
            print(f"\n  💬 Agent Response:")
            print(f"  {'-' * 50}")
            # Format response with wrapping
            words = response.split()
            line = "  "
            for word in words:
                if len(line) + len(word) > 70:
                    print(line)
                    line = "  "
                line += word + " "
            if line.strip():
                print(line)
            print(f"  {'-' * 50}")
            
            # Pause between scenarios
            await asyncio.sleep(1)
        
        print(f"\n{'=' * 70}")
        print("  DEMO COMPLETE")
        print(f"{'=' * 70}")
        
    finally:
        await agent.stop()
        
        if agent.action_logger:
            print(f"\n📁 Full log saved to: {agent.action_logger.log_file}")


async def run_reasoning_demo(model: str = "llama3.2", base_url: str = "http://localhost:11434"):
    """
    Run demo with DETAILED reasoning and decision output.
    
    Shows:
    - Full agent thinking process
    - Tool calls with parameters and results
    - Decision reasoning chain
    - Performance metrics
    """
    try:
        from rich.console import Console
        from rich.panel import Panel
        from rich.table import Table
        from rich.markdown import Markdown
        from rich.box import ROUNDED, DOUBLE, HEAVY
        from rich.syntax import Syntax
        from rich.tree import Tree
        from rich.text import Text
        HAS_RICH = True
    except ImportError:
        HAS_RICH = False
        print("Install 'rich' for enhanced output: pip install rich")
    
    from .ollama_demo import OllamaTradingAgent
    
    if HAS_RICH:
        console = Console()
    else:
        console = None
    
    def print_section(title: str, style: str = "cyan"):
        if console:
            console.print(Panel(f"[bold]{title}[/bold]", style=style, box=HEAVY))
        else:
            print(f"\n{'=' * 60}")
            print(f"  {title}")
            print(f"{'=' * 60}")
    
    def print_thinking(thought: str):
        if console:
            console.print(Panel(
                f"[italic blue]{thought}[/italic blue]",
                title="🧠 Agent Thinking",
                border_style="blue",
                box=ROUNDED,
            ))
        else:
            print(f"\n🧠 THINKING: {thought}")
    
    def print_tool_call(tool_name: str, params: dict, result: dict):
        if console:
            # Create tool call panel
            param_text = "\n".join([f"  • {k}: {v}" for k, v in params.items()])
            
            # Format result nicely
            if isinstance(result, dict):
                result_lines = []
                for k, v in result.items():
                    if isinstance(v, dict):
                        result_lines.append(f"  {k}:")
                        for k2, v2 in v.items():
                            result_lines.append(f"    • {k2}: {v2}")
                    elif isinstance(v, list):
                        result_lines.append(f"  {k}: [{len(v)} items]")
                    else:
                        result_lines.append(f"  {k}: {v}")
                result_text = "\n".join(result_lines[:15])  # Limit lines
                if len(result_lines) > 15:
                    result_text += "\n  ... (truncated)"
            else:
                result_text = str(result)[:500]
            
            content = f"[bold cyan]Parameters:[/bold cyan]\n{param_text}\n\n[bold green]Result:[/bold green]\n{result_text}"
            
            console.print(Panel(
                content,
                title=f"🔧 Tool: {tool_name}",
                border_style="green",
                box=ROUNDED,
            ))
        else:
            print(f"\n🔧 TOOL CALL: {tool_name}")
            print(f"   Params: {params}")
            print(f"   Result: {str(result)[:200]}...")
    
    def print_decision(decision: str, reasoning: str = None):
        if console:
            content = f"[bold yellow]{decision}[/bold yellow]"
            if reasoning:
                content += f"\n\n[dim]Reasoning:[/dim]\n{reasoning}"
            
            console.print(Panel(
                content,
                title="📊 Decision",
                border_style="yellow",
                box=DOUBLE,
            ))
        else:
            print(f"\n📊 DECISION: {decision}")
            if reasoning:
                print(f"   Reasoning: {reasoning}")
    
    # Header
    if console:
        console.print(Panel(
            "[bold magenta]NELEUS AI TRADING AGENT[/bold magenta]\n"
            "[dim]Detailed Reasoning & Decision Demo[/dim]\n\n"
            f"Model: [cyan]{model}[/cyan]",
            box=DOUBLE,
        ))
    else:
        print_banner()
        print(f"\n🤖 Detailed Reasoning Demo - Model: {model}")
    
    agent = OllamaTradingAgent(
        name="ReasoningDemo",
        model=model,
        base_url=base_url,
        instruments=["BTC", "ETH"],
        log_actions=True,
    )
    
    # Demo scenarios with expected reasoning
    scenarios = [
        {
            "name": "1. Market Volatility Analysis",
            "prompt": "Check the current volatility for BTC. Explain what regime we're in and what this means for trading strategy.",
            "expected_tools": ["MonitorVolatility"],
        },
        {
            "name": "2. Strategy Backtesting",
            "prompt": "Run a momentum backtest on BTC from 2025-01-01 to 2025-12-31. Analyze the key metrics and tell me if this strategy is viable.",
            "expected_tools": ["RunBacktest"],
        },
        {
            "name": "3. Risk Assessment",
            "prompt": "I want to take a 1.0 BTC position. Calculate risk metrics and advise on position sizing for a $50,000 portfolio.",
            "expected_tools": ["CalculateRiskMetrics"],
        },
        {
            "name": "4. Final Trading Decision",
            "prompt": "Based on all analysis, should I trade BTC now? Give me a clear BUY/HOLD/SELL recommendation with full reasoning.",
            "expected_tools": ["GetMarketRegime"],
        },
    ]
    
    try:
        await agent.start()
        
        all_tool_calls = []
        all_decisions = []
        
        for scenario in scenarios:
            print_section(scenario["name"])
            
            if console:
                console.print(f"\n[bold]📝 Prompt:[/bold] {scenario['prompt']}\n")
            else:
                print(f"\n📝 Prompt: {scenario['prompt']}\n")
            
            # Show thinking
            print_thinking(f"Processing request... Expected tools: {', '.join(scenario['expected_tools'])}")
            
            # Execute
            response = await agent.think_and_act(scenario["prompt"])
            
            # Show tool calls from action logger
            if agent.action_logger:
                recent_actions = agent.action_logger.actions[-5:]  # Last 5 actions
                
                for action in recent_actions:
                    if action.action_type == "tool_call" and action.tool_name:
                        print_tool_call(
                            action.tool_name,
                            action.parameters or {},
                            action.result or {},
                        )
                        all_tool_calls.append({
                            "tool": action.tool_name,
                            "success": action.success,
                            "duration": action.duration_ms,
                        })
            
            # Show decision/response
            print_decision(response[:300], reasoning=response[300:600] if len(response) > 300 else None)
            all_decisions.append(response[:100])
            
            if console:
                console.print()
            else:
                print()
            
            await asyncio.sleep(0.5)
        
        # Final Summary
        print_section("📊 SESSION SUMMARY", style="magenta")
        
        if console:
            # Summary table
            summary_table = Table(title="Tool Usage", box=ROUNDED)
            summary_table.add_column("Tool", style="cyan")
            summary_table.add_column("Status", justify="center")
            summary_table.add_column("Duration", justify="right")
            
            for tc in all_tool_calls:
                status = "[green]✓[/green]" if tc["success"] else "[red]✗[/red]"
                summary_table.add_row(
                    tc["tool"],
                    status,
                    f"{tc['duration']:.0f}ms" if tc["duration"] else "N/A",
                )
            
            console.print(summary_table)
            
            # Decision summary
            if agent.action_logger:
                summary = agent.action_logger.get_summary()
                
                stats_table = Table(title="Session Statistics", box=ROUNDED)
                stats_table.add_column("Metric", style="cyan")
                stats_table.add_column("Value", style="green", justify="right")
                
                stats_table.add_row("Total Actions", str(summary["total_actions"]))
                stats_table.add_row("Tool Calls", str(summary["tool_calls"]))
                stats_table.add_row("Success Rate", f"{summary['successful_tools']}/{summary['tool_calls']}")
                stats_table.add_row("Total Duration", f"{summary['total_duration_s']:.1f}s")
                stats_table.add_row("Unique Tools", ", ".join(summary["tools_used"]) or "None")
                
                console.print(stats_table)
                console.print(f"\n📁 Full log: [cyan]{agent.action_logger.log_file}[/cyan]")
        else:
            print(f"\nTool Calls: {len(all_tool_calls)}")
            print(f"Decisions Made: {len(all_decisions)}")
            if agent.action_logger:
                print(f"Log file: {agent.action_logger.log_file}")
        
    finally:
        await agent.stop()
        
        if console:
            console.print(Panel("[bold green]✓ Reasoning Demo Complete![/bold green]", border_style="green"))


async def run_visual_demo(model: str = "llama3.2", base_url: str = "http://localhost:11434"):
    """
    Run a demo with rich terminal visualization including:
    - Live order book display
    - Price charts
    - Agent activity feed
    - Performance metrics dashboard
    
    Requires the 'rich' package for enhanced output.
    """
    try:
        from rich.console import Console
        from rich.panel import Panel
        from rich.progress import Progress, SpinnerColumn, TextColumn
        from rich.table import Table
        from rich.live import Live
        from rich.layout import Layout
        from rich.markdown import Markdown
        from rich.box import DOUBLE, ROUNDED
        from rich.align import Align
        from rich.text import Text
        HAS_RICH = True
    except ImportError:
        HAS_RICH = False
        print("Note: Install 'rich' for enhanced visualization: pip install rich")
    
    if not HAS_RICH:
        await run_automated_demo(model, base_url)
        return
    
    from .ollama_demo import OllamaTradingAgent, ActionLogger
    from .rich_ui import TradingUI, OrderBookVisualizer, PriceChartVisualizer, MetricsDashboard, ActivityFeed
    
    console = Console()
    
    # Create trading UI components
    trading_ui = TradingUI(instrument="BTC")
    
    # Header panel
    def make_header():
        header_text = Text()
        header_text.append("🤖 ", style="bold")
        header_text.append("NELEUS AI TRADING AGENT", style="bold magenta")
        header_text.append(" | ", style="dim")
        header_text.append(f"Model: {model}", style="cyan")
        header_text.append(" | ", style="dim")
        header_text.append(f"{datetime.now().strftime('%H:%M:%S')}", style="green")
        return Panel(Align.center(header_text), box=DOUBLE, style="bold")
    
    console.print(make_header())
    console.print()
    
    # Create agent
    agent = OllamaTradingAgent(
        name="VisualDemo",
        model=model,
        base_url=base_url,
        instruments=["BTC", "ETH"],
        log_actions=True,
    )
    
    def create_full_layout():
        """Create the complete dashboard layout."""
        layout = Layout()
        
        # Main split: top (chart + orderbook) and bottom (activity + metrics)
        layout.split_column(
            Layout(name="top", ratio=2),
            Layout(name="bottom", ratio=1),
        )
        
        # Top: chart on left, orderbook on right
        layout["top"].split_row(
            Layout(name="chart", ratio=3),
            Layout(name="orderbook", ratio=2),
        )
        
        # Bottom: activity on left, metrics+positions on right
        layout["bottom"].split_row(
            Layout(name="activity", ratio=2),
            Layout(name="metrics", ratio=1),
            Layout(name="positions", ratio=1),
        )
        
        return layout
    
    def update_layout(layout: Layout, current_action: str = ""):
        """Update all panels in the layout."""
        # Generate fresh mock data for live feel
        trading_ui._generate_mock_data()
        
        # Update each panel
        layout["chart"].update(trading_ui.chart.render())
        layout["orderbook"].update(trading_ui.orderbook.render())
        layout["activity"].update(trading_ui.activity.render())
        layout["metrics"].update(trading_ui.metrics.render())
        layout["positions"].update(trading_ui.metrics.render_positions())
        
        return layout
    
    try:
        await agent.start()
        
        # Demo scenarios
        demo_scenarios = [
            {
                "name": "Market Regime Analysis",
                "prompt": "Check the current volatility for BTC. What's the market regime?",
                "action": "volatility_check",
            },
            {
                "name": "Backtest Strategy",
                "prompt": "Run a momentum backtest on BTC from 2025-01-01 to 2025-12-31.",
                "action": "backtest",
            },
            {
                "name": "Risk Assessment",
                "prompt": "Calculate risk metrics for a 1.0 BTC position.",
                "action": "risk_calc",
            },
            {
                "name": "Trading Decision",
                "prompt": "Based on the analysis, should I trade BTC? Give a recommendation.",
                "action": "decision",
            },
        ]
        
        layout = create_full_layout()
        
        # Add initial activity
        trading_ui.activity.add_thinking("Initializing trading agent...")
        
        # Shared state for background updates
        ui_running = True
        current_status = "Starting..."
        
        async def background_ui_updates(live):
            """Background task to update UI continuously."""
            while ui_running:
                live.update(update_layout(layout))
                await asyncio.sleep(0.2)  # 5 FPS for smooth updates
        
        async def run_scenarios():
            """Run through demo scenarios."""
            nonlocal current_status, ui_running
            
            for i, scenario in enumerate(demo_scenarios):
                # Show scenario start
                current_status = f"[{i+1}/{len(demo_scenarios)}] {scenario['name']}"
                trading_ui.activity.add_thinking(current_status)
                await asyncio.sleep(0.3)
                
                # Execute scenario
                trading_ui.activity.add_thinking(f"Processing: {scenario['prompt'][:40]}...")
                
                try:
                    response = await agent.think_and_act(scenario["prompt"])
                    
                    # Update UI based on action type
                    if scenario["action"] == "volatility_check":
                        trading_ui.activity.add_tool_call("MonitorVolatility", success=True, details="Regime analysis complete")
                        trading_ui.metrics.update({
                            "regime": "normal",
                            "risk_level": "medium",
                        })
                    elif scenario["action"] == "backtest":
                        trading_ui.activity.add_tool_call("RunBacktest", success=True, details="Momentum strategy tested")
                        trading_ui.metrics.update({
                            "trades": 45,
                            "win_rate": 58.5,
                            "sharpe": 1.82,
                        })
                    elif scenario["action"] == "risk_calc":
                        trading_ui.activity.add_tool_call("CalculateRiskMetrics", success=True, details="VaR calculated")
                        trading_ui.metrics.update({
                            "max_drawdown": 8.5,
                        })
                    elif scenario["action"] == "decision":
                        trading_ui.activity.add_decision(response[:60] + "..." if len(response) > 60 else response)
                    
                    # Update metrics from tool results if available
                    if agent.action_logger:
                        for action in agent.action_logger.actions[-3:]:
                            if action.action_type == "tool_call" and action.result:
                                result = action.result
                                if "volatility" in result:
                                    vol_data = result.get("volatility", {})
                                    trading_ui.metrics.update({
                                        "volatility": vol_data.get("realized_annualized_pct", 35),
                                        "regime": result.get("regime", "normal"),
                                    })
                                if "return_pct" in result:
                                    trading_ui.metrics.update({
                                        "pnl_pct": result.get("return_pct", 0),
                                        "sharpe": result.get("sharpe_ratio", 0),
                                        "max_drawdown": result.get("max_drawdown_pct", 0),
                                    })
                
                except Exception as e:
                    trading_ui.activity.add_tool_call("Error", success=False, details=str(e)[:40])
                
                await asyncio.sleep(0.5)
            
            # Final update
            trading_ui.activity.add_decision("Demo complete! All scenarios processed.")
            await asyncio.sleep(1.0)
            ui_running = False
        
        # Run with live display and background updates
        with Live(update_layout(layout), console=console, refresh_per_second=5, screen=False) as live:
            # Run both tasks concurrently
            ui_task = asyncio.create_task(background_ui_updates(live))
            scenario_task = asyncio.create_task(run_scenarios())
            
            # Wait for scenarios to complete
            await scenario_task
            ui_running = False
            
            # Cancel background task
            ui_task.cancel()
            try:
                await ui_task
            except asyncio.CancelledError:
                pass
        
        # Print summary
        console.print()
        
        if agent.action_logger:
            summary = agent.action_logger.get_summary()
            
            summary_table = Table(title="📊 Session Summary", box=ROUNDED)
            summary_table.add_column("Metric", style="cyan")
            summary_table.add_column("Value", style="green")
            
            summary_table.add_row("Total Actions", str(summary["total_actions"]))
            summary_table.add_row("Tool Calls", str(summary["tool_calls"]))
            summary_table.add_row("Successful", str(summary["successful_tools"]))
            summary_table.add_row("Duration", f"{summary['total_duration_s']:.1f}s")
            summary_table.add_row("Tools Used", ", ".join(summary["tools_used"]) or "None")
            
            console.print(summary_table)
            console.print(f"\n📁 Full log: {agent.action_logger.log_file}")
            
            # Generate HTML report
            try:
                from .visualization import HTMLReportGenerator
                html_gen = HTMLReportGenerator()
                report_path = html_gen.generate_report(agent.action_logger.actions, summary)
                console.print(f"📄 HTML Report: {report_path}")
            except Exception as e:
                pass
            
    finally:
        await agent.stop()
        console.print(Panel("[bold green]✓ Demo Complete![/bold green]", border_style="green"))


def main():
    """Main entry point for demo runner."""
    parser = argparse.ArgumentParser(
        description="Neleus AI Agent Demo - Powered by Ollama",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Demo Modes:
  --reasoning, -r     Detailed reasoning view with tool calls and decision process
  --charts, -c        Visual charts with order book, price chart, metrics dashboard
  --auto, -a          Basic automated demo (prompts and responses only)

Examples:
  %(prog)s --reasoning              # Show detailed reasoning and decisions
  %(prog)s --charts                 # Visual trading dashboard
  %(prog)s --reasoning -m llama3.1  # Use different model
  %(prog)s --charts -m mistral      # Charts with mistral model
        """,
    )
    
    parser.add_argument(
        "--model", "-m",
        default="llama3.2",
        help="Ollama model to use (default: llama3.2)",
    )
    
    parser.add_argument(
        "--base-url", "-u",
        default="http://localhost:11434",
        help="Ollama API base URL (default: http://localhost:11434)",
    )
    
    # Mutually exclusive demo modes
    mode_group = parser.add_mutually_exclusive_group()
    
    mode_group.add_argument(
        "--reasoning", "-r",
        action="store_true",
        help="Detailed reasoning view - shows thinking, tool calls, and decision process",
    )
    
    mode_group.add_argument(
        "--charts", "-c",
        action="store_true", 
        help="Visual charts view - order book, price chart, metrics dashboard",
    )
    
    mode_group.add_argument(
        "--auto", "-a",
        action="store_true",
        help="Basic automated demo - prompts and responses only",
    )
    
    mode_group.add_argument(
        "--visual", "-v",
        action="store_true",
        help="(Alias for --charts) Visual charts view",
    )
    
    parser.add_argument(
        "--no-banner",
        action="store_true",
        help="Skip the welcome banner",
    )
    
    args = parser.parse_args()
    
    # Print banner
    if not args.no_banner:
        print_banner()
    
    # Check Ollama
    print("Checking Ollama availability...")
    if not check_ollama():
        print("\n⚠️  Ollama is not running!")
        print("Please start Ollama first:")
        print("  1. Install: https://ollama.ai")
        print("  2. Run: ollama serve")
        print(f"  3. Pull model: ollama pull {args.model}")
        print("\nThen run this demo again.")
        sys.exit(1)
    
    print(f"✓ Ollama is running at {args.base_url}")
    print(f"✓ Using model: {args.model}")
    
    # Determine which mode to run
    if args.charts or args.visual:
        print("\n📊 Starting VISUAL CHARTS demo...\n")
        run_func = run_visual_demo
    elif args.reasoning:
        print("\n🧠 Starting DETAILED REASONING demo...\n")
        run_func = run_reasoning_demo
    elif args.auto:
        print("\n🤖 Starting AUTOMATED demo...\n")
        run_func = run_automated_demo
    else:
        # Default to interactive
        print("\n💬 Starting INTERACTIVE demo...\n")
        print("Choose a mode next time:")
        print("  --reasoning  : Detailed agent reasoning and decisions")
        print("  --charts     : Visual trading dashboard with charts")
        print()
        from .ollama_demo import run_interactive_demo
        run_func = run_interactive_demo
    
    # Run demo
    try:
        asyncio.run(run_func(args.model, args.base_url))
    except KeyboardInterrupt:
        print("\n\nDemo interrupted. Goodbye!")
    except Exception as e:
        logger.exception(f"Demo error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
