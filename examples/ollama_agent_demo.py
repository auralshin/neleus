#!/usr/bin/env python3
"""
Neleus AI Agent Demo with Ollama

This example demonstrates how to use Neleus AI trading agents powered by Ollama:
1. Market analysis using technical indicators
2. Running backtests
3. Monitoring volatility
4. Risk assessment
5. Making informed trading decisions

Prerequisites:
- Ollama installed and running (https://ollama.ai)
- Pull a model: ollama pull llama3.2
- Install neleus: pip install neleus (or maturin develop for local build)

Usage:
    python ollama_agent_demo.py
    python ollama_agent_demo.py --auto
    python ollama_agent_demo.py --model mistral
"""

import asyncio
import argparse
import sys
from datetime import datetime, timedelta
from pathlib import Path

# Add parent directory to path for local development
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

from neleus.ai import (
    OllamaTradingAgent,
    ActionLogger,
    run_interactive_demo,
)
from neleus.ai.visualization import (
    TerminalVisualizer,
    HTMLReportGenerator,
    create_demo_visualization,
)


async def run_demo_scenario(agent: OllamaTradingAgent) -> None:
    """Run a predefined demo scenario showcasing agent capabilities."""
    
    print("\n" + "=" * 70)
    print("  NELEUS AI AGENT DEMO - Powered by Ollama")
    print("  Demonstrating: Market Analysis, Backtesting, Risk Management")
    print("=" * 70)
    
    scenarios = [
        {
            "title": "1. Market Volatility Analysis",
            "description": "Analyzing current volatility regime for BTC",
            "prompt": "Monitor the current volatility for BTC. What regime are we in? Is it a good time to trade?",
        },
        {
            "title": "2. Market Regime Detection",
            "description": "Detecting overall market conditions",
            "prompt": "Analyze the market regime for BTC and ETH. Are we in a risk-on or risk-off environment?",
        },
        {
            "title": "3. Strategy Backtesting",
            "description": "Running a momentum strategy backtest",
            "prompt": "Run a momentum strategy backtest on BTC from 2025-01-01 to 2025-12-31. Analyze the results and tell me if this strategy is profitable.",
        },
        {
            "title": "4. Risk Assessment",
            "description": "Calculating risk metrics for a position",
            "prompt": "I want to take a position of 1.0 BTC. Calculate the risk metrics including VaR. Is this an appropriate position size for a $100,000 portfolio?",
        },
        {
            "title": "5. Trading Recommendation",
            "description": "Synthesizing analysis into a recommendation",
            "prompt": "Based on all our analysis - volatility, market regime, backtest results, and risk - should I be trading BTC right now? Give me a clear recommendation.",
        },
    ]
    
    for scenario in scenarios:
        print(f"\n{'─' * 70}")
        print(f"  {scenario['title']}")
        print(f"  {scenario['description']}")
        print(f"{'─' * 70}")
        print(f"\n  📝 Prompt: {scenario['prompt'][:80]}...")
        print()
        
        try:
            response = await agent.think_and_act(scenario["prompt"])
            
            print(f"\n  💬 Agent Response:")
            print(f"  {'-' * 60}")
            
            # Word wrap the response
            words = response.split()
            line = "  "
            for word in words:
                if len(line) + len(word) > 72:
                    print(line)
                    line = "  "
                line += word + " "
            if line.strip():
                print(line)
            
            print(f"  {'-' * 60}")
            
        except Exception as e:
            print(f"\n  ❌ Error: {e}")
        
        # Small pause between scenarios
        await asyncio.sleep(0.5)
    
    print(f"\n{'=' * 70}")
    print("  DEMO COMPLETE")
    print(f"{'=' * 70}")


async def main():
    """Main entry point for the demo."""
    parser = argparse.ArgumentParser(
        description="Neleus AI Agent Demo with Ollama",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python ollama_agent_demo.py              # Interactive mode
  python ollama_agent_demo.py --auto       # Automated demo
  python ollama_agent_demo.py --model mistral
  python ollama_agent_demo.py --generate-report  # Generate HTML report from logs
        """,
    )
    
    parser.add_argument(
        "--model", "-m",
        default="llama3.2",
        help="Ollama model to use (default: llama3.2)",
    )
    
    parser.add_argument(
        "--auto", "-a",
        action="store_true",
        help="Run automated demo sequence",
    )
    
    parser.add_argument(
        "--interactive", "-i",
        action="store_true",
        help="Run interactive chat mode",
    )
    
    parser.add_argument(
        "--generate-report",
        type=str,
        metavar="LOG_FILE",
        help="Generate HTML report from a log file",
    )
    
    args = parser.parse_args()
    
    # Check Ollama is running
    print("Checking Ollama availability...")
    try:
        import urllib.request
        req = urllib.request.Request("http://localhost:11434/api/tags")
        with urllib.request.urlopen(req, timeout=2) as resp:
            if resp.status != 200:
                raise Exception("Ollama not responding")
        print(f"✓ Ollama is running")
        print(f"✓ Using model: {args.model}")
    except Exception as e:
        print(f"\n⚠️  Ollama is not running!")
        print("Please start Ollama first:")
        print("  1. Install: https://ollama.ai")
        print("  2. Run: ollama serve")
        print(f"  3. Pull model: ollama pull {args.model}")
        sys.exit(1)
    
    # Generate report from log file
    if args.generate_report:
        log_path = Path(args.generate_report)
        if not log_path.exists():
            print(f"Log file not found: {log_path}")
            sys.exit(1)
        
        print(f"Generating report from: {log_path}")
        report_path = create_demo_visualization(log_path, output_format="html")
        if report_path:
            print(f"✓ Report generated: {report_path}")
        sys.exit(0)
    
    # Create agent
    agent = OllamaTradingAgent(
        name="DemoAgent",
        model=args.model,
        instruments=["BTC", "ETH", "SOL"],
        log_actions=True,
    )
    
    try:
        await agent.start()
        
        if args.interactive:
            # Interactive mode
            print("\n" + "=" * 60)
            print("  INTERACTIVE MODE")
            print("  Type your questions, 'quit' to exit, 'summary' for summary")
            print("=" * 60)
            
            while True:
                try:
                    user_input = input("\n🤖 You: ").strip()
                    
                    if not user_input:
                        continue
                    
                    if user_input.lower() == "quit":
                        break
                    
                    if user_input.lower() == "summary":
                        if agent.action_logger:
                            agent.action_logger.print_summary()
                        continue
                    
                    response = await agent.think_and_act(user_input)
                    print(f"\n💬 {agent.name}: {response}")
                    
                except KeyboardInterrupt:
                    print("\n\nInterrupted.")
                    break
        else:
            # Automated demo
            await run_demo_scenario(agent)
        
    finally:
        await agent.stop()
        
        # Generate visualization
        if agent.action_logger:
            # Terminal visualization
            viz = TerminalVisualizer()
            summary = agent.action_logger.get_summary()
            
            print(viz.draw_summary_card(
                agent_name=agent.name,
                duration_s=summary["total_duration_s"],
                tool_calls=summary["tool_calls"],
                decisions=summary["decisions"],
                success_rate=summary["successful_tools"] / max(1, summary["tool_calls"]) * 100,
            ))
            
            # Generate HTML report
            generator = HTMLReportGenerator()
            actions = [a.to_dict() for a in agent.action_logger.actions]
            report_path = generator.generate_report(agent.name, actions, summary)
            
            print(f"\n📁 Logs saved to: {agent.action_logger.log_file}")
            print(f"📊 HTML report: {report_path}")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n\nDemo interrupted. Goodbye!")
    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
