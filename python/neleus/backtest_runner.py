"""
Backtest Runner Module

Handles loading strategies and running backtests from user projects.
"""
import asyncio
import sys
import yaml
from pathlib import Path
from typing import Dict, Any, List, Optional, TYPE_CHECKING
from datetime import datetime
from decimal import Decimal

# Use relative imports to avoid circular dependencies
from .strategy import Strategy
from .node import HyperliquidBacktestConfig, HyperliquidBacktestNode, CandleInterval


class BacktestRunner:
    """Runs backtests for user strategies."""
    
    def __init__(self, project_path: Path):
        self.project_path = project_path
        self.strategies_dir = project_path / "strategies"
        
        # Support both config.yaml and neleus.toml
        self.config_path = project_path / "config.yaml"
        self.toml_config_path = project_path / "neleus.toml"
        
    def load_config(self) -> Dict[str, Any]:
        """Load project configuration from YAML or TOML."""
        if self.config_path.exists():
            with open(self.config_path) as f:
                return yaml.safe_load(f)
        elif self.toml_config_path.exists():
            try:
                import toml
                with open(self.toml_config_path) as f:
                    return toml.load(f)
            except ImportError:
                # Fallback to basic TOML parsing
                config = {}
                with open(self.toml_config_path) as f:
                    content = f.read()
                    # Very basic TOML parsing for common keys
                    for line in content.split('\n'):
                        if '=' in line and not line.strip().startswith('#'):
                            key, value = line.split('=', 1)
                            key = key.strip()
                            value = value.strip().strip('"').strip("'")
                            try:
                                value = float(value)
                            except:
                                pass
                            config[key] = value
                return config
        else:
            # Return default config
            return {
                "backtest": {
                    "initial_capital": 100000.0,
                    "candle_interval": "1h",
                    "maker_fee_bps": 2.0,
                    "taker_fee_bps": 5.0,
                    "slippage_bps": 5.0,
                },
                "instruments": [{"symbol": "BTC"}]
            }
    
    def load_strategy_config(self, strategy_name: str) -> Dict[str, Any]:
        """Load strategy-specific configuration."""
        config_path = self.project_path / "configs" / f"{strategy_name}.yaml"
        if config_path.exists():
            with open(config_path) as f:
                return yaml.safe_load(f)
        return {}
    
    def load_strategy_class(self, strategy_name: str):
        """Dynamically load strategy class from file."""
        # Try different naming conventions
        strategy_file = self.strategies_dir / f"{strategy_name}.py"
        if not strategy_file.exists():
            strategy_file = self.strategies_dir / f"{strategy_name}_strategy.py"
        if not strategy_file.exists():
            # Try without _strategy suffix
            base_name = strategy_name.replace("_strategy", "")
            strategy_file = self.strategies_dir / f"{base_name}_strategy.py"
        if not strategy_file.exists():
            raise FileNotFoundError(f"Strategy file not found: {strategy_name} (looked in {self.strategies_dir})")
        
        # Add strategies directory to Python path
        sys.path.insert(0, str(self.strategies_dir))
        
        try:
            # Import the module
            import importlib.util
            spec = importlib.util.spec_from_file_location(strategy_name, strategy_file)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            
            # Find the Strategy subclass
            strategy_class = None
            for attr_name in dir(module):
                attr = getattr(module, attr_name)
                if (isinstance(attr, type) and 
                    issubclass(attr, Strategy) and 
                    attr is not Strategy):
                    strategy_class = attr
                    break
            
            if strategy_class is None:
                raise ValueError(f"No Strategy subclass found in {strategy_file}")
            
            return strategy_class
        
        finally:
            # Clean up sys.path
            if str(self.strategies_dir) in sys.path:
                sys.path.remove(str(self.strategies_dir))
    
    def create_backtest_config(
        self,
        config: Dict[str, Any],
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        initial_capital: Optional[float] = None,
        coin: Optional[str] = None,
    ) -> HyperliquidBacktestConfig:
        """Create backtest configuration from project config."""
        backtest_cfg = config.get("backtest", {})
        
        # Parse interval
        interval_str = backtest_cfg.get("candle_interval", "1h")
        interval_map = {
            "1m": CandleInterval.MIN_1,
            "5m": CandleInterval.MIN_5,
            "15m": CandleInterval.MIN_15,
            "1h": CandleInterval.HOUR_1,
            "4h": CandleInterval.HOUR_4,
            "1d": CandleInterval.DAY_1,
        }
        interval = interval_map.get(interval_str, CandleInterval.HOUR_1)
        
        # Get coin from instruments
        # TODO: no default token, throw error is not specified
        if coin is None:
            instruments = config.get("instruments", [])
            if instruments:
                coin = instruments[0].get("symbol", "BTC")
            else:
                coin = "BTC"
        
        # Handle dates - if not specified, use lookback_days
        start_time = None
        end_time = None
        lookback_days = 30  # default
        
        if start_date and end_date:
            # Use explicit dates if both provided
            start_time = datetime.fromisoformat(start_date)
            end_time = datetime.fromisoformat(end_date)
        elif start_date or end_date:
            # If only one date provided, calculate from config defaults
            config_start = backtest_cfg.get("start_date")
            config_end = backtest_cfg.get("end_date")
            if config_start and config_end:
                start_time = datetime.fromisoformat(start_date or config_start)
                end_time = datetime.fromisoformat(end_date or config_end)
        # else: leave None to use lookback_days
        
        return HyperliquidBacktestConfig(
            coin=coin,
            interval=interval,
            start_time=start_time,
            end_time=end_time,
            lookback_days=lookback_days,
            testnet=False,
            initial_capital=Decimal(str(initial_capital or backtest_cfg.get("initial_capital", 10000))),
            maker_fee_bps=backtest_cfg.get("maker_fee_bps", 2.0),
            taker_fee_bps=backtest_cfg.get("taker_fee_bps", 5.0),
            slippage_bps=backtest_cfg.get("slippage_bps", 5.0),
        )
    
    async def run_backtest(
        self,
        strategy_name: Optional[str] = None,
        start_date: Optional[str] = None,
        end_date: Optional[str] = None,
        initial_capital: Optional[float] = None,
    ) -> Dict[str, Any]:
        """Run backtest for specified strategy or all strategies."""
        # Load project config
        config = self.load_config()
        
        # Determine which strategies to run
        if strategy_name:
            strategy_names = [strategy_name]
        else:
            # Run all strategies in the strategies directory
            strategy_names = []
            for f in self.strategies_dir.glob("*.py"):
                if not f.name.startswith("_") and f.name != "__init__.py":
                    strategy_names.append(f.stem)
        
        if not strategy_names:
            print("No strategies found to run.")
            return {}
        
        results = {}
        
        for strat_name in strategy_names:
            print(f"\n{'='*60}")
            print(f"Running backtest: {strat_name}")
            print('='*60)
            
            try:
                # Load strategy class
                StrategyClass = self.load_strategy_class(strat_name)
                
                # Load strategy config
                strat_config = self.load_strategy_config(strat_name)
                params = strat_config.get("strategy", {}).get("parameters", {})
                
                # Instantiate strategy
                strategy = StrategyClass(**params)
                
                # Create backtest config
                backtest_config = self.create_backtest_config(
                    config,
                    start_date=start_date,
                    end_date=end_date,
                    initial_capital=initial_capital,
                )
                
                # Create and run backtest node
                node = HyperliquidBacktestNode(backtest_config)
                node.add_strategy(strategy)
                
                result = await node.run_async()
                results[strat_name] = result
                
                # Print results
                self.print_results(result)
                
            except Exception as e:
                print(f"Error running backtest for {strat_name}: {e}")
                import traceback
                traceback.print_exc()
        
        return results
    
    def print_results(self, result):
        """Print backtest results."""
        # Use the built-in summary method from BacktestResults
        print(result.summary())
        
        # Additional info
        if hasattr(result, 'equity_curve') and result.equity_curve:
            initial = result.equity_curve[0][1]
            final = result.equity_curve[-1][1]
            print(f"Equity: ${initial:,.2f} → ${final:,.2f}")
            print()
        
        if hasattr(result, 'fills'):
            print(f"Total trades: {len(result.fills)}")
            if result.fills and len(result.fills) > 0:
                print("Sample trades:")
                for fill in result.fills[:5]:
                    print(f"  qty={fill.get('quantity', 0):.4f} @ ${fill.get('price', 0):,.2f} (fee: ${fill.get('commission', 0):.4f})")
            print()


def run_backtest_sync(
    project_path: Optional[Path] = None,
    strategy_name: Optional[str] = None,
    start_date: Optional[str] = None,
    end_date: Optional[str] = None,
    initial_capital: Optional[float] = None,
) -> Dict[str, Any]:
    """Synchronous wrapper for running backtests."""
    if project_path is None:
        project_path = Path.cwd()
    
    runner = BacktestRunner(project_path)
    return asyncio.run(runner.run_backtest(
        strategy_name=strategy_name,
        start_date=start_date,
        end_date=end_date,
        initial_capital=initial_capital,
    ))
