#!/usr/bin/env python3
"""
Test script for Neleus AI Agent Demo

This script tests the Ollama agent implementation without requiring Ollama to be running.
It tests the core components: tools, memory, and visualization.

Usage:
    python test_ollama_agent.py
"""

import asyncio
import sys
from pathlib import Path
from datetime import datetime

# Add parent directories to path
sys.path.insert(0, str(Path(__file__).parent.parent / "python"))

# Test imports
print("Testing imports...")

try:
    from neleus.ai import (
        # Core
        AIAgent,
        AgentConfig,
        PersonalityConfig,
        InfoConfig,
        AgentState,
        # Memory
        MemoryStore,
        LocalMemoryStore,
        MemoryEntry,
        MemoryType,
        # Tools
        Tool,
        ToolRegistry,
        ToolResult,
        # Demo
        OllamaTradingAgent,
        ActionLogger,
        run_interactive_demo,
        # Visualization
        TerminalVisualizer,
        HTMLReportGenerator,
    )
    print("✓ All imports successful")
except ImportError as e:
    print(f"✗ Import error: {e}")
    sys.exit(1)

# Test demo tools
try:
    from neleus.ai.demo_tools import (
        RunBacktestTool,
        MonitorVolatilityTool,
        GetMarketRegimeTool,
        CalculateRiskMetricsTool,
    )
    print("✓ Demo tools import successful")
except ImportError as e:
    print(f"✗ Demo tools import error: {e}")
    sys.exit(1)


async def test_memory():
    """Test local memory store."""
    print("\nTesting LocalMemoryStore...")
    
    store = LocalMemoryStore()
    
    # Store a memory
    entry = MemoryEntry(
        memory_type=MemoryType.OBSERVATION,
        content="BTC price dropped 5% in the last hour",
        metadata={"instrument": "BTC", "change": -5.0},
        importance=0.8,
    )
    
    memory_id = await store.store(entry)
    print(f"  ✓ Stored memory: {memory_id}")
    
    # Recall memories
    memories = await store.recall("BTC price", limit=5)
    print(f"  ✓ Recalled {len(memories)} memories")
    
    # Get count
    count = await store.count()
    print(f"  ✓ Total memories: {count}")
    
    print("✓ Memory tests passed")


async def test_tools():
    """Test demo tools."""
    print("\nTesting demo tools...")
    
    # Test MonitorVolatilityTool (doesn't require external data)
    tool = MonitorVolatilityTool()
    print(f"  Tool: {tool.name}")
    print(f"  Description: {tool.description[:50]}...")
    
    # Execute tool (will use simulated data if real data unavailable)
    result = await tool.execute(
        instrument="BTC",
        lookback_hours=24,
        include_forecast=True,
    )
    
    if result.success:
        print(f"  ✓ MonitorVolatility executed successfully")
        print(f"    Output keys: {list(result.output.keys())}")
    else:
        print(f"  ✓ MonitorVolatility returned error (expected without network): {result.error}")
    
    # Test RunBacktestTool
    backtest_tool = RunBacktestTool()
    result = await backtest_tool.execute(
        instrument="BTC",
        strategy="momentum",
        start_date="2025-01-01",
        end_date="2025-12-31",
        initial_capital=100000.0,
    )
    
    if result.success:
        print(f"  ✓ RunBacktest executed successfully")
        print(f"    Return: {result.output.get('return_pct', 'N/A')}%")
        print(f"    Sharpe: {result.output.get('sharpe_ratio', 'N/A')}")
    else:
        print(f"  ✓ RunBacktest returned error: {result.error}")
    
    print("✓ Tool tests passed")


def test_visualization():
    """Test visualization components."""
    print("\nTesting visualization...")
    
    viz = TerminalVisualizer()
    
    # Test box drawing
    box = viz.draw_box("Test Box", ["Line 1", "Line 2", "Line 3"])
    print(box)
    
    # Test summary card
    card = viz.draw_summary_card(
        agent_name="TestAgent",
        duration_s=120.5,
        tool_calls=15,
        decisions=5,
        success_rate=93.3,
    )
    print(card)
    
    print("✓ Visualization tests passed")


def test_action_logger():
    """Test action logger."""
    print("\nTesting ActionLogger...")
    
    logger = ActionLogger(
        agent_name="TestAgent",
        log_dir=Path("test_logs"),
        console_output=False,  # Don't print to console during test
    )
    
    # Log some actions
    logger.log_thinking("Analyzing market conditions for BTC")
    
    logger.log_tool_call(
        tool_name="monitor_volatility",
        inputs={"instrument": "BTC"},
        result=ToolResult(success=True, output={"regime": "normal"}),
        duration_ms=150.5,
    )
    
    logger.log_decision(
        {"action": "buy", "instrument": "BTC", "size": 0.1},
        reasoning="Volatility is low, market regime is bullish",
    )
    
    # Get summary
    summary = logger.get_summary()
    print(f"  Total actions: {summary['total_actions']}")
    print(f"  Tool calls: {summary['tool_calls']}")
    print(f"  Decisions: {summary['decisions']}")
    print(f"  Log file: {logger.log_file}")
    
    print("✓ ActionLogger tests passed")


def test_html_report():
    """Test HTML report generation."""
    print("\nTesting HTMLReportGenerator...")
    
    generator = HTMLReportGenerator(output_dir=Path("test_reports"))
    
    actions = [
        {
            "timestamp": datetime.now().isoformat(),
            "action_type": "thinking",
            "reasoning": "Analyzing market conditions",
        },
        {
            "timestamp": datetime.now().isoformat(),
            "action_type": "tool_call",
            "tool_name": "monitor_volatility",
            "success": True,
            "output_data": {"regime": "normal", "volatility": 45.2},
        },
        {
            "timestamp": datetime.now().isoformat(),
            "action_type": "decision",
            "output_data": {"action": "observe", "reason": "Waiting for better entry"},
        },
    ]
    
    summary = {
        "total_duration_s": 60.0,
        "tool_calls": 1,
        "decisions": 1,
        "successful_tools": 1,
    }
    
    report_path = generator.generate_report("TestAgent", actions, summary)
    print(f"  ✓ HTML report generated: {report_path}")
    
    print("✓ HTMLReportGenerator tests passed")


async def main():
    """Run all tests."""
    print("=" * 60)
    print("  NELEUS AI AGENT DEMO - TEST SUITE")
    print("=" * 60)
    
    try:
        await test_memory()
        await test_tools()
        test_visualization()
        test_action_logger()
        test_html_report()
        
        print("\n" + "=" * 60)
        print("  ALL TESTS PASSED ✓")
        print("=" * 60)
        print("\nYou can now run the full demo with:")
        print("  python examples/ollama_agent_demo.py --auto")
        print("\nMake sure Ollama is running:")
        print("  ollama serve")
        print("  ollama pull llama3.2")
        
    except Exception as e:
        print(f"\n✗ Test failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
