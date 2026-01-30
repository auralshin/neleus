"""
Neleus Managed Agent API
========================

Python interface for managing always-on trading agents.

Example:
    from neleus.agents import AgentManager, AgentSpec, VenueSpec
    
    manager = AgentManager("http://localhost:8080")
    
    # Deploy a new agent
    spec = AgentSpec(
        name="BTC Momentum Bot",
        strategy_id="momentum_strategy",
        strategy_config={"lookback": 20, "threshold": 0.02},
        venue=VenueSpec.hyperliquid(network="mainnet"),
        instruments=["BTC-PERP"],
        capital={"initial": 10000, "default_leverage": 2},
    )
    
    agent_id = manager.deploy(spec)
    manager.start(agent_id)
    
    # Monitor the agent
    stats = manager.get_stats(agent_id)
    print(f"P&L: {stats.realized_pnl}")
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Any, Dict, List, Optional, Callable
import json
import requests


class AgentState(Enum):
    """Agent execution state."""
    CREATED = "created"
    INITIALIZING = "initializing"
    READY = "ready"
    RUNNING = "running"
    PAUSED = "paused"
    STOPPING = "stopping"
    STOPPED = "stopped"
    ERROR = "error"
    UPGRADING = "upgrading"


@dataclass
class VenueSpec:
    """Venue configuration for an agent."""
    venue_type: str
    network: str = "mainnet"
    wallet_address: Optional[str] = None
    use_vault: bool = False
    slippage_bps: float = 0.0
    latency_ms: int = 0
    
    @classmethod
    def hyperliquid(
        cls,
        network: str = "mainnet",
        wallet_address: Optional[str] = None,
        use_vault: bool = False,
    ) -> "VenueSpec":
        """Create Hyperliquid venue spec."""
        return cls(
            venue_type="Hyperliquid",
            network=network,
            wallet_address=wallet_address,
            use_vault=use_vault,
        )
    
    @classmethod
    def lighter(cls, network: str = "mainnet") -> "VenueSpec":
        """Create Lighter venue spec."""
        return cls(venue_type="Lighter", network=network)
    
    @classmethod
    def simulated(cls, slippage_bps: float = 0.0, latency_ms: int = 0) -> "VenueSpec":
        """Create simulated venue spec."""
        return cls(
            venue_type="Simulated",
            slippage_bps=slippage_bps,
            latency_ms=latency_ms,
        )
    
    def to_dict(self) -> Dict[str, Any]:
        if self.venue_type == "Simulated":
            return {
                "type": self.venue_type,
                "slippage_bps": self.slippage_bps,
                "latency_ms": self.latency_ms,
            }
        else:
            return {
                "type": self.venue_type,
                "network": self.network,
                "wallet_address": self.wallet_address,
                "use_vault": self.use_vault,
            }


@dataclass
class RiskLimits:
    """Risk limits for an agent."""
    max_position_size: Optional[float] = None
    max_notional: Optional[float] = None
    max_total_exposure: Optional[float] = None
    max_daily_loss: Optional[float] = None
    max_drawdown_pct: Optional[float] = None
    max_orders_per_second: Optional[int] = None
    max_open_orders: Optional[int] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {k: v for k, v in {
            "max_position_size": self.max_position_size,
            "max_notional": self.max_notional,
            "max_total_exposure": self.max_total_exposure,
            "max_daily_loss": self.max_daily_loss,
            "max_drawdown_pct": self.max_drawdown_pct,
            "max_orders_per_second": self.max_orders_per_second,
            "max_open_orders": self.max_open_orders,
        }.items() if v is not None}


@dataclass
class CapitalSpec:
    """Capital allocation for an agent."""
    initial: float
    maximum: Optional[float] = None
    default_leverage: float = 1.0
    currency: str = "USD"
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "initial": self.initial,
            "maximum": self.maximum,
            "default_leverage": self.default_leverage,
            "currency": self.currency,
        }


@dataclass
class ScheduleSpec:
    """Operating schedule for an agent."""
    start: str  # Cron expression
    stop: str   # Cron expression
    timezone: str = "UTC"
    blackout_dates: List[str] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "start": self.start,
            "stop": self.stop,
            "timezone": self.timezone,
            "blackout_dates": self.blackout_dates,
        }


@dataclass
class SignalSourceSpec:
    """External signal source specification."""
    id: str
    source_type: str  # "webhook", "redis", "kafka", "grpc", "signal_hub"
    config: Dict[str, Any] = field(default_factory=dict)
    filters: Dict[str, str] = field(default_factory=dict)
    
    @classmethod
    def webhook(cls, id: str, secret: Optional[str] = None) -> "SignalSourceSpec":
        """Create webhook signal source."""
        return cls(id=id, source_type="Webhook", config={"secret": secret})
    
    @classmethod
    def signal_hub(cls, id: str, signal_type: str) -> "SignalSourceSpec":
        """Create signal hub source."""
        return cls(id=id, source_type="SignalHub", config={"signal_type": signal_type})
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "id": self.id,
            "type": self.source_type,
            **self.config,
            "filters": self.filters,
        }


@dataclass
class AgentSpec:
    """Agent deployment specification."""
    name: str
    strategy_id: str
    venue: VenueSpec
    instruments: List[str]
    
    # Optional fields
    agent_id: Optional[str] = None
    strategy_config: Dict[str, Any] = field(default_factory=dict)
    risk_limits: Optional[RiskLimits] = None
    capital: Optional[CapitalSpec] = None
    schedule: Optional[ScheduleSpec] = None
    signal_sources: List[SignalSourceSpec] = field(default_factory=list)
    labels: Dict[str, str] = field(default_factory=dict)
    
    # Environment
    paper_trading: bool = False
    log_level: str = "info"
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "agent_id": self.agent_id,
            "name": self.name,
            "strategy_id": self.strategy_id,
            "strategy_config": self.strategy_config,
            "venue": self.venue.to_dict(),
            "instruments": self.instruments,
            "risk_limits": self.risk_limits.to_dict() if self.risk_limits else {},
            "capital": self.capital.to_dict() if self.capital else {"initial": 10000},
            "schedule": self.schedule.to_dict() if self.schedule else None,
            "signal_sources": [s.to_dict() for s in self.signal_sources],
            "labels": self.labels,
            "environment": {
                "paper_trading": self.paper_trading,
                "log_level": self.log_level,
            },
        }


@dataclass
class AgentStats:
    """Agent statistics."""
    agent_id: str
    state: AgentState
    created_at: datetime
    started_at: Optional[datetime]
    uptime_seconds: int
    orders_placed: int
    trades_executed: int
    realized_pnl: float
    unrealized_pnl: float
    restart_count: int
    last_error: Optional[str]
    
    @property
    def total_pnl(self) -> float:
        return self.realized_pnl + self.unrealized_pnl
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "AgentStats":
        return cls(
            agent_id=data["agent_id"],
            state=AgentState(data["state"]),
            created_at=datetime.fromisoformat(data["created_at"]),
            started_at=datetime.fromisoformat(data["started_at"]) if data.get("started_at") else None,
            uptime_seconds=data.get("uptime_seconds", 0),
            orders_placed=data.get("orders_placed", 0),
            trades_executed=data.get("trades_executed", 0),
            realized_pnl=data.get("realized_pnl", 0.0),
            unrealized_pnl=data.get("unrealized_pnl", 0.0),
            restart_count=data.get("restart_count", 0),
            last_error=data.get("last_error"),
        )


class AgentManager:
    """
    Manager for deploying and controlling trading agents.
    
    Example:
        manager = AgentManager("http://localhost:8080")
        
        # Deploy and start an agent
        spec = AgentSpec(
            name="My Bot",
            strategy_id="my_strategy",
            venue=VenueSpec.hyperliquid(),
            instruments=["BTC-PERP"],
        )
        agent_id = manager.deploy(spec)
        manager.start(agent_id)
        
        # Monitor
        stats = manager.get_stats(agent_id)
        print(f"Running: {stats.state == AgentState.RUNNING}")
    """
    
    def __init__(
        self,
        orchestrator_url: str = "http://localhost:8080",
        api_key: Optional[str] = None,
        timeout: int = 30,
    ):
        self.orchestrator_url = orchestrator_url.rstrip("/")
        self.api_key = api_key
        self.timeout = timeout
        self._session = requests.Session()
        
        if api_key:
            self._session.headers["Authorization"] = f"Bearer {api_key}"
    
    def deploy(self, spec: AgentSpec) -> str:
        """
        Deploy a new trading agent.
        
        Args:
            spec: Agent specification
            
        Returns:
            Agent ID
        """
        response = self._session.post(
            f"{self.orchestrator_url}/api/v1/agents",
            json=spec.to_dict(),
            timeout=self.timeout,
        )
        response.raise_for_status()
        return response.json()["agent_id"]
    
    def start(self, agent_id: str) -> None:
        """Start a trading agent."""
        response = self._session.post(
            f"{self.orchestrator_url}/api/v1/agents/{agent_id}/start",
            timeout=self.timeout,
        )
        response.raise_for_status()
    
    def stop(self, agent_id: str, force: bool = False) -> None:
        """
        Stop a trading agent.
        
        Args:
            agent_id: Agent ID
            force: Force immediate stop without graceful shutdown
        """
        response = self._session.post(
            f"{self.orchestrator_url}/api/v1/agents/{agent_id}/stop",
            json={"force": force},
            timeout=self.timeout,
        )
        response.raise_for_status()
    
    def pause(self, agent_id: str) -> None:
        """Pause a trading agent (keeps positions, stops new trades)."""
        response = self._session.post(
            f"{self.orchestrator_url}/api/v1/agents/{agent_id}/pause",
            timeout=self.timeout,
        )
        response.raise_for_status()
    
    def resume(self, agent_id: str) -> None:
        """Resume a paused agent."""
        response = self._session.post(
            f"{self.orchestrator_url}/api/v1/agents/{agent_id}/resume",
            timeout=self.timeout,
        )
        response.raise_for_status()
    
    def upgrade(self, agent_id: str, new_spec: AgentSpec) -> None:
        """
        Upgrade an agent to a new specification (rolling update).
        
        Args:
            agent_id: Agent ID
            new_spec: New agent specification
        """
        response = self._session.put(
            f"{self.orchestrator_url}/api/v1/agents/{agent_id}",
            json=new_spec.to_dict(),
            timeout=self.timeout,
        )
        response.raise_for_status()
    
    def remove(self, agent_id: str) -> None:
        """Remove an agent completely."""
        response = self._session.delete(
            f"{self.orchestrator_url}/api/v1/agents/{agent_id}",
            timeout=self.timeout,
        )
        response.raise_for_status()
    
    def get_stats(self, agent_id: str) -> AgentStats:
        """Get agent statistics."""
        response = self._session.get(
            f"{self.orchestrator_url}/api/v1/agents/{agent_id}/stats",
            timeout=self.timeout,
        )
        response.raise_for_status()
        return AgentStats.from_dict(response.json())
    
    def list_agents(self) -> List[AgentStats]:
        """List all agents."""
        response = self._session.get(
            f"{self.orchestrator_url}/api/v1/agents",
            timeout=self.timeout,
        )
        response.raise_for_status()
        return [AgentStats.from_dict(a) for a in response.json()]
    
    def get_logs(
        self,
        agent_id: str,
        lines: int = 100,
        since: Optional[datetime] = None,
    ) -> List[str]:
        """Get agent logs."""
        params = {"lines": lines}
        if since:
            params["since"] = since.isoformat()
        
        response = self._session.get(
            f"{self.orchestrator_url}/api/v1/agents/{agent_id}/logs",
            params=params,
            timeout=self.timeout,
        )
        response.raise_for_status()
        return response.json()["logs"]
    
    def health_check(self) -> bool:
        """Check if orchestrator is healthy."""
        try:
            response = self._session.get(
                f"{self.orchestrator_url}/health",
                timeout=5,
            )
            return response.status_code == 200
        except:
            return False


# Convenience class for local development/testing
class LocalAgentRunner:
    """
    Run agents locally for development and testing.
    
    Example:
        from neleus.agents import LocalAgentRunner
        from my_strategies import MomentumStrategy
        
        runner = LocalAgentRunner()
        runner.add_strategy("momentum", MomentumStrategy())
        runner.run(instruments=["BTC-PERP"])
    """
    
    def __init__(self):
        self._strategies: Dict[str, Any] = {}
        self._running = False
    
    def add_strategy(self, strategy_id: str, strategy: Any) -> None:
        """Register a strategy for local execution."""
        self._strategies[strategy_id] = strategy
    
    def run(
        self,
        strategy_id: Optional[str] = None,
        instruments: Optional[List[str]] = None,
        paper_trading: bool = True,
        initial_capital: float = 10000.0,
    ) -> None:
        """
        Run an agent locally.
        
        Args:
            strategy_id: Strategy to run (uses first if not specified)
            instruments: Instruments to trade
            paper_trading: Use paper trading mode
            initial_capital: Starting capital
        """
        if not strategy_id and self._strategies:
            strategy_id = list(self._strategies.keys())[0]
        
        if strategy_id not in self._strategies:
            raise ValueError(f"Strategy not found: {strategy_id}")
        
        strategy = self._strategies[strategy_id]
        
        # Import here to avoid circular imports
        from .node import PaperNode, PaperConfig, SimulationConfig
        from decimal import Decimal
        
        config = PaperConfig(
            simulation=SimulationConfig(initial_capital=Decimal(str(initial_capital))),
        )
        
        node = PaperNode(config)
        node.add_strategy(strategy)
        
        if instruments:
            for instrument in instruments:
                node.subscribe(instrument)
        
        print(f"Running {strategy_id} locally...")
        self._running = True
        
        try:
            node.run()
        except KeyboardInterrupt:
            print("\nStopping...")
        finally:
            self._running = False
