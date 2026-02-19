"""
Neleus Configuration
====================

Loads and validates project configuration from YAML files and environment
variables. Provides structured dataclasses for venues, instruments, risk,
backtest, dashboard, and logging settings.
"""

import os
from pathlib import Path
from typing import Any, Dict, List, Optional
from dataclasses import dataclass, field
from decimal import Decimal

try:
    import yaml
    HAS_YAML = True
except ImportError:
    HAS_YAML = False
    yaml = None


@dataclass
class VenueConfig:
    """Configuration for a single trading venue."""

    name: str
    enabled: bool = True
    mode: str = "paper"  # "backtest", "paper", or "live"
    testnet: bool = True

    # Credentials (populated from environment)
    private_key: Optional[str] = None
    api_key: Optional[str] = None
    api_secret: Optional[str] = None


@dataclass
class InstrumentConfig:
    """Configuration for a tradeable instrument."""

    symbol: str
    venue: str
    instrument_type: str = "perp"


@dataclass
class RiskConfig:
    """Risk management limits for a project."""

    max_position_size: Decimal = Decimal("1.0")
    max_notional_value: Decimal = Decimal("10000")
    max_drawdown_pct: float = 10.0
    daily_loss_limit: Decimal = Decimal("500")
    max_orders_per_second: int = 10
    max_open_orders: int = 50


@dataclass
class BacktestConfig:
    """Configuration for backtest runs."""

    start_date: str = "2024-01-01"
    end_date: str = "2024-12-31"
    initial_capital: Decimal = Decimal("10000")
    base_currency: str = "USDC"
    slippage_bps: int = 5
    maker_fee_bps: int = 2
    taker_fee_bps: int = 5
    candle_interval: str = "1h"


@dataclass
class DashboardConfig:
    """Web dashboard configuration."""

    port: int = 8080
    host: str = "127.0.0.1"
    auto_open_browser: bool = True
    default_timeframe: str = "1h"
    max_candles: int = 1000
    price_refresh_ms: int = 1000
    orderbook_refresh_ms: int = 500
    position_refresh_ms: int = 2000


@dataclass
class LoggingConfig:
    """Logging configuration."""

    level: str = "INFO"
    format: str = "%(asctime)s | %(levelname)s | %(name)s | %(message)s"
    file_enabled: bool = True
    file_path: str = "logs/neleus.log"
    max_file_size_mb: int = 10
    backup_count: int = 5


@dataclass
class ProjectConfig:
    """Top-level project configuration."""

    name: str
    version: str = "0.1.0"
    description: str = ""

    venues: Dict[str, VenueConfig] = field(default_factory=dict)
    instruments: List[Dict[str, Any]] = field(default_factory=list)
    risk: RiskConfig = field(default_factory=RiskConfig)
    backtest: BacktestConfig = field(default_factory=BacktestConfig)
    dashboard: DashboardConfig = field(default_factory=DashboardConfig)
    logging: LoggingConfig = field(default_factory=LoggingConfig)


# =============================================================================
# Config I/O
# =============================================================================

def load_env(project_path: Optional[Path] = None) -> Dict[str, str]:
    """Load .env file from *project_path* (or cwd) into os.environ."""
    env_vars: Dict[str, str] = {}

    env_path = (project_path / ".env") if project_path else (Path.cwd() / ".env")

    if not env_path.exists():
        return env_vars

    with open(env_path) as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if "=" in line:
                key, _, value = line.partition("=")
                key = key.strip()
                value = value.strip()
                if value and value[0] in ('"', "'") and value[-1] == value[0]:
                    value = value[1:-1]
                env_vars[key] = value
                os.environ[key] = value

    return env_vars


def load_project_config(config_path: Optional[Path] = None) -> Dict[str, Any]:
    """Load project configuration from a YAML file."""
    if not HAS_YAML:
        raise ImportError("PyYAML is required: pip install pyyaml")

    if config_path is None:
        config_path = Path.cwd() / "config.yaml"

    if not config_path.exists():
        raise FileNotFoundError(f"Configuration file not found: {config_path}")

    with open(config_path) as f:
        config = yaml.safe_load(f)

    load_env(config_path.parent)
    config = _apply_env_overrides(config)

    return config


def load_strategy_config(config_path: Path) -> Dict[str, Any]:
    """Load strategy-specific configuration from a YAML file."""
    if not HAS_YAML:
        raise ImportError("PyYAML is required: pip install pyyaml")

    if not config_path.exists():
        return {}

    with open(config_path) as f:
        return yaml.safe_load(f)


def _apply_env_overrides(config: Dict[str, Any]) -> Dict[str, Any]:
    """Apply NELEUS_* environment variable overrides to *config*."""
    if "risk" not in config:
        config["risk"] = {}

    env_mappings = {
        "NELEUS_MAX_POSITION_SIZE": ("risk", "max_position_size", float),
        "NELEUS_MAX_DRAWDOWN_PCT": ("risk", "max_drawdown_pct", float),
        "NELEUS_DAILY_LOSS_LIMIT": ("risk", "daily_loss_limit", float),
        "NELEUS_LOG_LEVEL": ("logging", "level", str),
        "NELEUS_DASHBOARD_PORT": ("dashboard", "port", int),
    }

    for env_key, (section, key, type_func) in env_mappings.items():
        if env_key in os.environ:
            if section not in config:
                config[section] = {}
            config[section][key] = type_func(os.environ[env_key])

    return config


def save_config(config: Dict[str, Any], config_path: Optional[Path] = None) -> None:
    """Persist *config* to a YAML file."""
    if not HAS_YAML:
        raise ImportError("PyYAML is required: pip install pyyaml")

    if config_path is None:
        config_path = Path.cwd() / "config.yaml"

    with open(config_path, "w") as f:
        yaml.dump(config, f, default_flow_style=False, sort_keys=False)


def validate_config(config: Dict[str, Any]) -> List[str]:
    """Validate *config* and return a list of error strings (empty = valid)."""
    errors: List[str] = []

    if "project" not in config:
        errors.append("Missing 'project' section")
    elif "name" not in config.get("project", {}):
        errors.append("Missing 'project.name'")

    venues = config.get("venues", {})
    for venue_name, venue_config in venues.items():
        if venue_config.get("enabled") and venue_config.get("mode") == "live":
            if venue_name == "hyperliquid":
                if not os.environ.get("HYPERLIQUID_PRIVATE_KEY"):
                    errors.append(
                        f"Venue '{venue_name}' in live mode requires HYPERLIQUID_PRIVATE_KEY"
                    )
            elif venue_name == "lighter":
                if not os.environ.get("LIGHTER_API_KEY"):
                    errors.append(
                        f"Venue '{venue_name}' in live mode requires LIGHTER_API_KEY"
                    )

    risk = config.get("risk", {})
    if risk.get("max_drawdown_pct", 0) <= 0:
        errors.append("risk.max_drawdown_pct must be positive")
    if risk.get("max_position_size", 0) <= 0:
        errors.append("risk.max_position_size must be positive")

    return errors


# =============================================================================
# Strategy discovery
# =============================================================================

def discover_strategies(project_path: Optional[Path] = None) -> List[Dict[str, Any]]:
    """Discover strategy files in *project_path*/strategies/."""
    if project_path is None:
        project_path = Path.cwd()

    strategies_dir = project_path / "strategies"
    if not strategies_dir.exists():
        return []

    strategies: List[Dict[str, Any]] = []
    for strategy_file in strategies_dir.glob("*.py"):
        if strategy_file.name.startswith("_"):
            continue

        strategy_name = strategy_file.stem

        config_file = project_path / "configs" / f"{strategy_name}.yaml"
        config: Dict[str, Any] = {}
        if config_file.exists() and HAS_YAML:
            with open(config_file) as f:
                config = yaml.safe_load(f) or {}

        strategies.append(
            {
                "name": strategy_name,
                "file": str(strategy_file),
                "config_file": str(config_file) if config_file.exists() else None,
                "enabled": config.get("strategy", {}).get("enabled", True),
                "class": config.get("strategy", {}).get(
                    "class", strategy_name.title().replace("_", "")
                ),
            }
        )

    return strategies


# =============================================================================
# JSON Schema (for external validation tooling)
# =============================================================================

CONFIG_SCHEMA = {
    "type": "object",
    "properties": {
        "project": {
            "type": "object",
            "properties": {
                "name": {"type": "string", "minLength": 1},
                "version": {"type": "string"},
                "description": {"type": "string"},
            },
            "required": ["name"],
        },
        "venues": {
            "type": "object",
            "additionalProperties": {
                "type": "object",
                "properties": {
                    "enabled": {"type": "boolean"},
                    "mode": {"type": "string", "enum": ["backtest", "paper", "live"]},
                },
            },
        },
        "risk": {
            "type": "object",
            "properties": {
                "max_position_size": {"type": "number", "minimum": 0},
                "max_drawdown_pct": {"type": "number", "minimum": 0, "maximum": 100},
                "daily_loss_limit": {"type": "number", "minimum": 0},
            },
        },
        "backtest": {
            "type": "object",
            "properties": {
                "start_date": {"type": "string", "format": "date"},
                "end_date": {"type": "string", "format": "date"},
                "initial_capital": {"type": "number", "minimum": 0},
            },
        },
    },
}

# =============================================================================
# Deployment Configuration (Managed Service)
# =============================================================================

from .deployment import (
    # Enums
    DeploymentEnvironment,
    # Config classes
    DeploymentConfig,
    SecretRef,
    DatabaseConfig,
    TelemetryConfig,
    ServiceConfig,
    AlertingConfig,
    TEEConfig,
    # Preset configs
    local_dev_config,
    docker_config,
    kubernetes_config,
    tee_config,
)

__all__ = [
    # Dataclasses
    "VenueConfig",
    "InstrumentConfig",
    "RiskConfig",
    "BacktestConfig",
    "DashboardConfig",
    "LoggingConfig",
    "ProjectConfig",
    # I/O
    "load_env",
    "load_project_config",
    "load_strategy_config",
    "save_config",
    "validate_config",
    "discover_strategies",
    # Schema
    "CONFIG_SCHEMA",
    # Deployment
    "DeploymentEnvironment",
    "DeploymentConfig",
    "SecretRef",
    "DatabaseConfig",
    "TelemetryConfig",
    "ServiceConfig",
    "AlertingConfig",
    "TEEConfig",
    "local_dev_config",
    "docker_config",
    "kubernetes_config",
    "tee_config",
]
