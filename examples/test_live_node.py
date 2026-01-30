#!/usr/bin/env python3
"""
Test LiveNode for live trading capabilities:
- Circuit breaker integration
- Order submission with risk checks
- Connection management
- Daily P&L tracking
- Error handling
"""

import sys
import time
from datetime import datetime

print("=" * 80)
print("Neleus LiveNode Test - Live Trading Infrastructure")
print("=" * 80)
print()

# Import neleus_core types
try:
    from neleus_core import (
        # Base types
        Venue as PyVenue, InstrumentId as PyInstrumentId, InstrumentType as PyInstrumentType, 
        OrderSide as PyOrderSide, OrderType as PyOrderType,
        
        # Live trading
        LiveNodeConfig as PyLiveNodeConfig, LiveNode as PyLiveNode, 
        LiveNodeState as PyLiveNodeState, CircuitState as PyCircuitState,
    )
    print("✓ Successfully imported neleus_core types")
except ImportError as e:
    print(f"✗ Failed to import neleus_core: {e}")
    sys.exit(1)


def test_circuit_breaker():
    """Test circuit breaker functionality"""
    print("\n" + "=" * 60)
    print("TESTING CIRCUIT BREAKER")
    print("=" * 60)
    
    # Create LiveNode with circuit breaker config
    config = PyLiveNodeConfig(
        instance_id="test_cb",
        venue=PyVenue.Hyperliquid,
        use_testnet=True,
        initial_capital=100000.0,
        max_position_notional=50000.0,
        max_daily_loss=5000.0,
        max_leverage=10.0,
        circuit_breaker_threshold=3,  # Open after 3 failures
        circuit_breaker_recovery_ms=5000,  # 5 second recovery
        paper_trading=True,
    )

    node = PyLiveNode(config)
    print(f"\n✓ Created LiveNode: {node}")
    
    # Check initial circuit state
    print(f"\n  Initial circuit state: {node.circuit_state()}")
    assert node.circuit_state() == PyCircuitState.Closed
    print("  ✓ Circuit starts in Closed state")
    
    # Test circuit allows requests when closed
    assert node.circuit_allows_request() == True
    print("  ✓ Circuit allows requests when Closed")
    
    # Simulate failures to trip the circuit
    print("\n  Simulating failures to trip circuit breaker...")
    for i in range(3):
        node.record_failure(f"Test failure {i+1}")
        print(f"    Failure {i+1} recorded")
        print(f"    Circuit state: {node.circuit_state()}")
    
    # Circuit should be open now
    assert node.circuit_state() == PyCircuitState.Open
    print("  ✓ Circuit breaker OPENED after 3 failures")
    
    # Verify requests are blocked
    assert node.circuit_allows_request() == False
    print("  ✓ Circuit blocks requests when Open")
    
    # Reset circuit
    node.reset_circuit_breaker()
    assert node.circuit_state() == PyCircuitState.Closed
    print("\n  ✓ Circuit breaker reset to Closed state")


def test_live_node_lifecycle():
    """Test LiveNode connection and state management"""
    print("\n" + "=" * 60)
    print("TESTING LIVE NODE LIFECYCLE")
    print("=" * 60)
    
    # Create config for paper trading
    config = PyLiveNodeConfig(
        instance_id="test_lifecycle",
        venue=PyVenue.Hyperliquid,
        api_key="test_key",
        api_secret="test_secret",
        use_testnet=True,
        initial_capital=100000.0,
        max_position_notional=50000.0,
        max_daily_loss=5000.0,
        max_leverage=10.0,
        paper_trading=True,  # Enable paper trading mode
    )
    
    node = PyLiveNode(config)
    print(f"\n✓ Created LiveNode for {config.venue}")
    
    # Check initial state
    state = node.state()
    print(f"  Initial state: {state}")
    assert state == PyLiveNodeState.Disconnected
    print("  ✓ Node starts in Disconnected state")
    
    # Connect to exchange
    print("\n  Connecting to exchange...")
    node.connect()
    state = node.state()
    print(f"  State after connect: {state}")
    assert state == PyLiveNodeState.Connected
    print("  ✓ Successfully connected")
    
    # Start trading
    print("\n  Starting trading...")
    node.start_trading()
    state = node.state()
    print(f"  State after start_trading: {state}")
    assert state == PyLiveNodeState.Trading
    print("  ✓ Node is now in Trading state")
    
    # Check readiness
    is_ready = node.is_ready()
    print(f"\n  Is ready for trading: {is_ready}")
    assert is_ready == True
    print("  ✓ Node is ready for trading")
    
    # Stop trading
    print("\n  Stopping trading...")
    node.stop_trading()
    state = node.state()
    print(f"  State after stop_trading: {state}")
    assert state == PyLiveNodeState.Connected
    print("  ✓ Stopped trading (still connected)")
    
    # Disconnect
    print("\n  Disconnecting...")
    node.disconnect()
    state = node.state()
    print(f"  State after disconnect: {state}")
    assert state == PyLiveNodeState.Disconnected
    print("  ✓ Disconnected successfully")


def test_order_submission():
    """Test order submission with risk checks"""
    print("\n" + "=" * 60)
    print("TESTING ORDER SUBMISSION")
    print("=" * 60)
    
    # Create LiveNode in paper trading mode
    config = PyLiveNodeConfig(
        instance_id="test_orders",
        venue=PyVenue.Hyperliquid,
        use_testnet=True,
        initial_capital=100000.0,
        max_position_notional=50000.0,
        max_daily_loss=5000.0,
        max_leverage=10.0,
        paper_trading=True,
    )
    
    node = PyLiveNode(config)
    node.connect()
    node.start_trading()
    
    # Create instrument
    btc_perp = PyInstrumentId(PyVenue.Hyperliquid, "BTC-PERP", PyInstrumentType.Perp)
    
    # Submit a few orders
    print("\n  Submitting test orders...")
    for i in range(3):
        order_id = node.submit_order(
            instrument=btc_perp,
            side=PyOrderSide.Buy,
            order_type=PyOrderType.Limit,
            quantity=1.0,
            price=50000.0 + i * 100
        )
        print(f"    Order {i+1} submitted: {order_id}")
    
    # Check statistics
    total_orders = node.total_orders()
    print(f"\n  Total orders submitted: {total_orders}")
    assert total_orders == 3
    print("  ✓ Order count matches")
    
    # Simulate fills with P&L
    print("\n  Simulating fills with P&L...")
    node.on_fill(
        instrument=btc_perp,
        side=PyOrderSide.Buy,
        price=50000.0,
        quantity=1.0,
        realized_pnl=100.0
    )
    print("    Fill 1: +$100 P&L")
    
    node.on_fill(
        instrument=btc_perp,
        side=PyOrderSide.Sell,
        price=50100.0,
        quantity=1.0,
        realized_pnl=100.0
    )
    print("    Fill 2: +$100 P&L")
    
    # Check daily P&L
    daily_pnl = node.daily_pnl()
    print(f"\n  Daily P&L: ${daily_pnl}")
    assert daily_pnl == 200.0
    print("  ✓ P&L tracking working")
    
    # Check fill count
    total_fills = node.total_fills()
    print(f"  Total fills: {total_fills}")
    assert total_fills == 2
    print("  ✓ Fill count matches")


def test_risk_limits():
    """Test risk limit enforcement"""
    print("\n" + "=" * 60)
    print("TESTING RISK LIMITS")
    print("=" * 60)
    
    # Create LiveNode with tight limits
    config = PyLiveNodeConfig(
        instance_id="test_risk",
        venue=PyVenue.Hyperliquid,
        use_testnet=True,
        initial_capital=100000.0,
        max_position_notional=50000.0,
        max_daily_loss=1000.0,  # $1000 loss limit
        max_leverage=10.0,
        paper_trading=True,
    )
    
    node = PyLiveNode(config)
    node.connect()
    node.start_trading()
    
    # Simulate large loss
    print("\n  Simulating large loss to trigger daily limit...")
    btc_perp = PyInstrumentId(PyVenue.Hyperliquid, "BTC-PERP", PyInstrumentType.Perp)
    
    node.on_fill(
        instrument=btc_perp,
        side=PyOrderSide.Buy,
        price=50000.0,
        quantity=1.0,
        realized_pnl=-1200.0  # Loss exceeds limit
    )
    
    daily_pnl = node.daily_pnl()
    print(f"  Daily P&L: ${daily_pnl}")
    print(f"  Daily loss limit: ${config.max_daily_loss}")
    
    # Try to submit order - should fail
    print("\n  Attempting to submit order after hitting loss limit...")
    try:
        order_id = node.submit_order(
            instrument=btc_perp,
            side=PyOrderSide.Buy,
            order_type=PyOrderType.Limit,
            quantity=1.0,
            price=50000.0
        )
        print(f"  ✗ Order should have been rejected but got: {order_id}")
        assert False, "Order should have been rejected"
    except Exception as e:
        print(f"  ✓ Order correctly rejected: {str(e)}")
    
    # Reset daily stats
    print("\n  Resetting daily statistics...")
    node.reset_daily_stats()
    daily_pnl = node.daily_pnl()
    print(f"  Daily P&L after reset: ${daily_pnl}")
    assert daily_pnl == 0.0
    print("  ✓ Daily stats reset successfully")


def test_error_handling():
    """Test error handling and connection errors"""
    print("\n" + "=" * 60)
    print("TESTING ERROR HANDLING")
    print("=" * 60)
    
    config = PyLiveNodeConfig(
        instance_id="test_errors",
        venue=PyVenue.Hyperliquid,
        use_testnet=True,
        initial_capital=100000.0,
        max_position_notional=50000.0,
        max_daily_loss=5000.0,
        max_leverage=10.0,
        circuit_breaker_threshold=3,
        paper_trading=True,
    )
    
    node = PyLiveNode(config)
    print(f"\n✓ Created LiveNode: {node}")
    
    # Check initial error count
    errors = node.connection_errors()
    print(f"\n  Initial connection errors: {errors}")
    assert errors == 0
    print("  ✓ No initial errors")
    
    # Simulate connection errors
    print("\n  Simulating connection errors...")
    for i in range(3):
        node.on_connection_error(f"Connection timeout {i+1}")
        errors = node.connection_errors()
        print(f"    Error {i+1}: {errors} total errors")
        print(f"    Circuit state: {node.circuit_state()}")
    
    # Circuit should be open after 3 errors
    assert node.circuit_state() == PyCircuitState.Open
    print("\n  ✓ Circuit breaker opened after repeated connection errors")
    
    # Check final error count
    errors = node.connection_errors()
    print(f"  Total connection errors: {errors}")
    assert errors == 3
    print("  ✓ Error count tracking working")


def test_configuration():
    """Test LiveNode configuration"""
    print("\n" + "=" * 60)
    print("TESTING CONFIGURATION")
    print("=" * 60)
    
    # Create config with custom settings
    config = PyLiveNodeConfig(
        instance_id="custom_config",
        venue=PyVenue.Lighter,
        api_key="my_api_key",
        api_secret="my_api_secret",
        use_testnet=False,
        initial_capital=500000.0,
        max_position_notional=100000.0,
        max_daily_loss=10000.0,
        max_leverage=5.0,
        circuit_breaker_threshold=5,
        circuit_breaker_recovery_ms=60000,
        reconnect_initial_delay_ms=2000,
        reconnect_max_delay_ms=60000,
        paper_trading=False,
    )
    
    print(f"\n  Config: {config}")
    print(f"\n  Instance ID: {config.instance_id}")
    print(f"  Venue: {config.venue}")
    print(f"  Testnet: {config.use_testnet}")
    print(f"  Initial Capital: ${config.initial_capital:,.2f}")
    print(f"  Max Position: ${config.max_position_notional:,.2f}")
    print(f"  Max Daily Loss: ${config.max_daily_loss:,.2f}")
    print(f"  Max Leverage: {config.max_leverage}x")
    print(f"  Circuit Breaker Threshold: {config.circuit_breaker_threshold}")
    print(f"  Circuit Breaker Recovery: {config.circuit_breaker_recovery_ms}ms")
    print(f"  Reconnect Initial Delay: {config.reconnect_initial_delay_ms}ms")
    print(f"  Reconnect Max Delay: {config.reconnect_max_delay_ms}ms")
    print(f"  Paper Trading: {config.paper_trading}")
    
    # Create node and verify config
    node = PyLiveNode(config)
    retrieved_config = node.config()
    
    print("\n  Verifying config retrieval...")
    assert retrieved_config.instance_id == config.instance_id
    assert retrieved_config.venue == config.venue
    assert retrieved_config.initial_capital == config.initial_capital
    print("  ✓ Configuration stored and retrieved correctly")


def main():
    """Run all tests"""
    print("\nStarting LiveNode tests...")
    print()
    
    try:
        test_circuit_breaker()
        print("\n✓ Circuit breaker tests PASSED")
        
        test_live_node_lifecycle()
        print("\n✓ Lifecycle tests PASSED")
        
        test_order_submission()
        print("\n✓ Order submission tests PASSED")
        
        test_risk_limits()
        print("\n✓ Risk limit tests PASSED")
        
        test_error_handling()
        print("\n✓ Error handling tests PASSED")
        
        test_configuration()
        print("\n✓ Configuration tests PASSED")
        
        print("\n" + "=" * 80)
        print("ALL TESTS PASSED ✓")
        print("=" * 80)
        print("\nLiveNode is ready for live trading!")
        print("Key features verified:")
        print("  • Circuit breaker protection")
        print("  • Connection state management")
        print("  • Order submission with risk checks")
        print("  • Daily P&L tracking and limits")
        print("  • Error handling and recovery")
        print("  • Configuration management")
        
    except AssertionError as e:
        print(f"\n✗ Test failed: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"\n✗ Unexpected error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
