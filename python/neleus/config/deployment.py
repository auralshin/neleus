"""
Neleus Deployment Configuration
===============================

Configuration schemas for deploying and managing trading agents
in various environments (local, cloud, TEE).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Union
import json
import yaml


class DeploymentEnvironment(Enum):
    """Deployment environment types."""
    LOCAL = "local"
    DOCKER = "docker"
    KUBERNETES = "kubernetes"
    TEE = "tee"  # Trusted Execution Environment
    ON_PREMISE = "on_premise"


class SecretBackend(Enum):
    """Secret storage backends."""
    ENV = "env"
    FILE = "file"
    AWS_SECRETS = "aws_secrets"
    GCP_SECRETS = "gcp_secrets"
    AZURE_KEYVAULT = "azure_keyvault"
    HASHICORP_VAULT = "hashicorp_vault"


@dataclass
class SecretRef:
    """Reference to a secret value."""
    backend: SecretBackend
    key: str
    version: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "backend": self.backend.value,
            "key": self.key,
            "version": self.version,
        }


@dataclass
class DatabaseConfig:
    """Database configuration for persistence."""
    type: str = "sqlite"  # sqlite, postgres, timescale
    connection_string: Optional[str] = None
    connection_secret: Optional[SecretRef] = None
    max_connections: int = 10
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "type": self.type,
            "connection_string": self.connection_string,
            "connection_secret": self.connection_secret.to_dict() if self.connection_secret else None,
            "max_connections": self.max_connections,
        }


@dataclass
class TelemetryConfig:
    """Telemetry and observability configuration."""
    enabled: bool = True
    metrics_port: int = 9090
    traces_enabled: bool = False
    traces_endpoint: Optional[str] = None
    log_level: str = "info"
    log_format: str = "json"
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "enabled": self.enabled,
            "metrics_port": self.metrics_port,
            "traces_enabled": self.traces_enabled,
            "traces_endpoint": self.traces_endpoint,
            "log_level": self.log_level,
            "log_format": self.log_format,
        }


@dataclass
class ServiceConfig:
    """Configuration for individual services."""
    enabled: bool = True
    port: int = 8080
    host: str = "0.0.0.0"
    replicas: int = 1
    resources: Optional[Dict[str, Any]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "enabled": self.enabled,
            "port": self.port,
            "host": self.host,
            "replicas": self.replicas,
            "resources": self.resources,
        }


@dataclass
class AlertingConfig:
    """Alerting configuration."""
    enabled: bool = True
    
    # Notification channels
    slack_webhook: Optional[str] = None
    slack_webhook_secret: Optional[SecretRef] = None
    discord_webhook: Optional[str] = None
    telegram_bot_token: Optional[SecretRef] = None
    telegram_chat_id: Optional[str] = None
    webhook_url: Optional[str] = None
    
    # Default alert rules
    enable_default_rules: bool = True
    max_drawdown_threshold: float = 0.1
    daily_loss_threshold: float = 0.05
    latency_threshold_ms: float = 1000.0
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "enabled": self.enabled,
            "slack_webhook": self.slack_webhook,
            "discord_webhook": self.discord_webhook,
            "telegram_chat_id": self.telegram_chat_id,
            "webhook_url": self.webhook_url,
            "enable_default_rules": self.enable_default_rules,
            "max_drawdown_threshold": self.max_drawdown_threshold,
            "daily_loss_threshold": self.daily_loss_threshold,
            "latency_threshold_ms": self.latency_threshold_ms,
        }


@dataclass
class TEEConfig:
    """Trusted Execution Environment configuration."""
    enabled: bool = False
    provider: str = "sgx"  # sgx, sev, nitro
    attestation_enabled: bool = True
    sealed_storage_path: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "enabled": self.enabled,
            "provider": self.provider,
            "attestation_enabled": self.attestation_enabled,
            "sealed_storage_path": self.sealed_storage_path,
        }


@dataclass
class DeploymentConfig:
    """
    Complete deployment configuration for Neleus.
    
    Example:
        config = DeploymentConfig(
            environment=DeploymentEnvironment.DOCKER,
            orchestrator=ServiceConfig(port=8080),
            signal_hub=ServiceConfig(port=8082),
            monitor=ServiceConfig(port=8083),
        )
        config.save("deployment.yaml")
    """
    # Environment
    environment: DeploymentEnvironment = DeploymentEnvironment.LOCAL
    name: str = "neleus"
    namespace: str = "default"
    
    # Services
    orchestrator: ServiceConfig = field(default_factory=lambda: ServiceConfig(port=8080))
    signal_hub: ServiceConfig = field(default_factory=lambda: ServiceConfig(port=8082))
    monitor: ServiceConfig = field(default_factory=lambda: ServiceConfig(port=8083))
    dashboard: ServiceConfig = field(default_factory=lambda: ServiceConfig(port=3000))
    
    # Database
    database: DatabaseConfig = field(default_factory=DatabaseConfig)
    
    # Telemetry
    telemetry: TelemetryConfig = field(default_factory=TelemetryConfig)
    
    # Alerting
    alerting: AlertingConfig = field(default_factory=AlertingConfig)
    
    # TEE (for privacy-preserving deployment)
    tee: TEEConfig = field(default_factory=TEEConfig)
    
    # Venue credentials
    venue_credentials: Dict[str, SecretRef] = field(default_factory=dict)
    
    # Additional settings
    max_agents: int = 100
    auto_restart_agents: bool = True
    graceful_shutdown_timeout: int = 30
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "environment": self.environment.value,
            "name": self.name,
            "namespace": self.namespace,
            "services": {
                "orchestrator": self.orchestrator.to_dict(),
                "signal_hub": self.signal_hub.to_dict(),
                "monitor": self.monitor.to_dict(),
                "dashboard": self.dashboard.to_dict(),
            },
            "database": self.database.to_dict(),
            "telemetry": self.telemetry.to_dict(),
            "alerting": self.alerting.to_dict(),
            "tee": self.tee.to_dict(),
            "venue_credentials": {k: v.to_dict() for k, v in self.venue_credentials.items()},
            "max_agents": self.max_agents,
            "auto_restart_agents": self.auto_restart_agents,
            "graceful_shutdown_timeout": self.graceful_shutdown_timeout,
        }
    
    def save(self, path: Union[str, Path], format: str = "yaml") -> None:
        """Save configuration to file."""
        path = Path(path)
        data = self.to_dict()
        
        with open(path, "w") as f:
            if format == "yaml":
                yaml.dump(data, f, default_flow_style=False, sort_keys=False)
            else:
                json.dump(data, f, indent=2)
    
    @classmethod
    def load(cls, path: Union[str, Path]) -> "DeploymentConfig":
        """Load configuration from file."""
        path = Path(path)
        
        with open(path) as f:
            if path.suffix in [".yaml", ".yml"]:
                data = yaml.safe_load(f)
            else:
                data = json.load(f)
        
        return cls.from_dict(data)
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "DeploymentConfig":
        """Create config from dictionary."""
        services = data.get("services", {})
        
        return cls(
            environment=DeploymentEnvironment(data.get("environment", "local")),
            name=data.get("name", "neleus"),
            namespace=data.get("namespace", "default"),
            orchestrator=ServiceConfig(**services.get("orchestrator", {})) if services.get("orchestrator") else ServiceConfig(port=8080),
            signal_hub=ServiceConfig(**services.get("signal_hub", {})) if services.get("signal_hub") else ServiceConfig(port=8082),
            monitor=ServiceConfig(**services.get("monitor", {})) if services.get("monitor") else ServiceConfig(port=8083),
            dashboard=ServiceConfig(**services.get("dashboard", {})) if services.get("dashboard") else ServiceConfig(port=3000),
            database=DatabaseConfig(**data.get("database", {})) if data.get("database") else DatabaseConfig(),
            telemetry=TelemetryConfig(**data.get("telemetry", {})) if data.get("telemetry") else TelemetryConfig(),
            alerting=AlertingConfig(**data.get("alerting", {})) if data.get("alerting") else AlertingConfig(),
            tee=TEEConfig(**data.get("tee", {})) if data.get("tee") else TEEConfig(),
            max_agents=data.get("max_agents", 100),
            auto_restart_agents=data.get("auto_restart_agents", True),
            graceful_shutdown_timeout=data.get("graceful_shutdown_timeout", 30),
        )


# Preset configurations

def local_dev_config() -> DeploymentConfig:
    """Configuration for local development."""
    return DeploymentConfig(
        environment=DeploymentEnvironment.LOCAL,
        name="neleus-dev",
        database=DatabaseConfig(type="sqlite", connection_string="./data/neleus.db"),
        telemetry=TelemetryConfig(log_level="debug", log_format="pretty"),
        alerting=AlertingConfig(enabled=False),
    )


def docker_config() -> DeploymentConfig:
    """Configuration for Docker deployment."""
    return DeploymentConfig(
        environment=DeploymentEnvironment.DOCKER,
        name="neleus",
        database=DatabaseConfig(
            type="postgres",
            connection_string="postgres://postgres:postgres@db:5432/neleus",
        ),
        telemetry=TelemetryConfig(log_format="json"),
    )


def kubernetes_config(namespace: str = "trading") -> DeploymentConfig:
    """Configuration for Kubernetes deployment."""
    return DeploymentConfig(
        environment=DeploymentEnvironment.KUBERNETES,
        name="neleus",
        namespace=namespace,
        orchestrator=ServiceConfig(replicas=2, resources={"cpu": "500m", "memory": "512Mi"}),
        signal_hub=ServiceConfig(replicas=2, resources={"cpu": "250m", "memory": "256Mi"}),
        monitor=ServiceConfig(replicas=1, resources={"cpu": "250m", "memory": "256Mi"}),
        database=DatabaseConfig(
            type="timescale",
            connection_secret=SecretRef(backend=SecretBackend.AWS_SECRETS, key="neleus/db-connection"),
        ),
        telemetry=TelemetryConfig(
            traces_enabled=True,
            traces_endpoint="http://jaeger:14268/api/traces",
        ),
    )


def tee_config() -> DeploymentConfig:
    """Configuration for TEE (Trusted Execution Environment) deployment."""
    return DeploymentConfig(
        environment=DeploymentEnvironment.TEE,
        name="neleus-secure",
        tee=TEEConfig(
            enabled=True,
            provider="sgx",
            attestation_enabled=True,
            sealed_storage_path="/sealed/data",
        ),
        database=DatabaseConfig(type="sqlite", connection_string="/sealed/data/neleus.db"),
        venue_credentials={
            "hyperliquid": SecretRef(backend=SecretBackend.FILE, key="/sealed/secrets/hyperliquid.json"),
        },
    )
