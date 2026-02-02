"""
Agent-to-Agent Communication

Provides:
- Message bus for pub/sub and direct messaging
- Structured message types for coordination
- Agent discovery and routing
"""

from __future__ import annotations

import asyncio
import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Any, Callable, Dict, List, Optional, Set
import uuid

logger = logging.getLogger(__name__)


class MessageType(str, Enum):
    """Types of agent messages."""
    DATA_REQUEST = "data_request"      # Request market data or analysis
    DATA_RESPONSE = "data_response"    # Response to data request
    SIGNAL_SHARE = "signal_share"      # Share a trading signal
    COORDINATION = "coordination"      # Coordinate actions between agents
    ALERT = "alert"                    # Urgent notification
    STATUS = "status"                  # Status update
    HEARTBEAT = "heartbeat"            # Keep-alive message


class MessagePriority(str, Enum):
    """Message priority levels."""
    CRITICAL = "critical"  # Immediate processing
    HIGH = "high"          # High priority
    NORMAL = "normal"      # Normal priority
    LOW = "low"            # Background processing


@dataclass
class AgentMessage:
    """A message between agents."""
    from_agent: str
    to_agent: str  # Agent ID or "*" for broadcast
    message_type: MessageType
    payload: Dict[str, Any]
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    timestamp: datetime = field(default_factory=datetime.now)
    correlation_id: Optional[str] = None  # For request/response matching
    priority: int = 5  # 1 (highest) to 10 (lowest)
    ttl_seconds: int = 300  # Time to live
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {
            "id": self.id,
            "from_agent": self.from_agent,
            "to_agent": self.to_agent,
            "message_type": self.message_type.value,
            "payload": self.payload,
            "timestamp": self.timestamp.isoformat(),
            "correlation_id": self.correlation_id,
            "priority": self.priority,
            "ttl_seconds": self.ttl_seconds,
        }
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "AgentMessage":
        """Create from dictionary."""
        return cls(
            id=data.get("id", str(uuid.uuid4())),
            from_agent=data["from_agent"],
            to_agent=data["to_agent"],
            message_type=MessageType(data["message_type"]),
            payload=data["payload"],
            timestamp=datetime.fromisoformat(data["timestamp"]) if data.get("timestamp") else datetime.now(),
            correlation_id=data.get("correlation_id"),
            priority=data.get("priority", 5),
            ttl_seconds=data.get("ttl_seconds", 300),
        )
    
    def is_expired(self) -> bool:
        """Check if message has expired."""
        age = (datetime.now() - self.timestamp).total_seconds()
        return age > self.ttl_seconds


# Type alias for message handler callbacks
MessageHandler = Callable[[AgentMessage], None]


class MessageBus(ABC):
    """
    Abstract message bus for agent communication.
    
    Supports:
    - Direct messaging (agent-to-agent)
    - Broadcast messaging (agent-to-all)
    - Topic-based pub/sub
    - Request/response patterns
    """
    
    @abstractmethod
    async def subscribe(
        self,
        agent_id: str,
        callback: MessageHandler,
        topics: Optional[List[str]] = None,
    ) -> None:
        """
        Subscribe an agent to receive messages.
        
        Args:
            agent_id: The subscribing agent's ID
            callback: Async function to call when message received
            topics: Optional list of topics to subscribe to
        """
        pass
    
    @abstractmethod
    async def unsubscribe(self, agent_id: str) -> None:
        """Unsubscribe an agent from all messages."""
        pass
    
    @abstractmethod
    async def publish(self, message: AgentMessage) -> None:
        """
        Publish a message to a specific agent.
        
        The message's to_agent field specifies the recipient.
        """
        pass
    
    @abstractmethod
    async def broadcast(self, message: AgentMessage) -> None:
        """
        Broadcast a message to all subscribed agents.
        
        Sets to_agent to "*" automatically.
        """
        pass
    
    @abstractmethod
    async def request(
        self,
        message: AgentMessage,
        timeout: float = 30.0,
    ) -> Optional[AgentMessage]:
        """
        Send a request and wait for a response.
        
        Uses correlation_id to match request/response.
        
        Args:
            message: The request message
            timeout: Seconds to wait for response
        
        Returns:
            Response message or None if timeout
        """
        pass
    
    @abstractmethod
    async def get_agents(self) -> List[str]:
        """Get list of all registered agent IDs."""
        pass


class LocalMessageBus(MessageBus):
    """
    In-process message bus for local agent communication.
    
    Suitable for:
    - Development and testing
    - Single-process deployments
    - Agents running in the same Python process
    """
    
    def __init__(self):
        self._subscribers: Dict[str, MessageHandler] = {}
        self._topic_subscribers: Dict[str, Set[str]] = {}
        self._pending_requests: Dict[str, asyncio.Future] = {}
        self._lock = asyncio.Lock()
    
    async def subscribe(
        self,
        agent_id: str,
        callback: MessageHandler,
        topics: Optional[List[str]] = None,
    ) -> None:
        """Subscribe an agent."""
        async with self._lock:
            self._subscribers[agent_id] = callback
            
            if topics:
                for topic in topics:
                    if topic not in self._topic_subscribers:
                        self._topic_subscribers[topic] = set()
                    self._topic_subscribers[topic].add(agent_id)
            
            logger.debug(f"Agent {agent_id} subscribed to message bus")
    
    async def unsubscribe(self, agent_id: str) -> None:
        """Unsubscribe an agent."""
        async with self._lock:
            if agent_id in self._subscribers:
                del self._subscribers[agent_id]
            
            for topic_subs in self._topic_subscribers.values():
                topic_subs.discard(agent_id)
            
            logger.debug(f"Agent {agent_id} unsubscribed from message bus")
    
    async def publish(self, message: AgentMessage) -> None:
        """Publish a message to a specific agent."""
        if message.is_expired():
            logger.warning(f"Message {message.id} expired, not delivering")
            return
        
        to_agent = message.to_agent
        
        # Check if this is a response to a pending request
        if message.correlation_id and message.correlation_id in self._pending_requests:
            self._pending_requests[message.correlation_id].set_result(message)
            return
        
        # Direct message
        if to_agent in self._subscribers:
            try:
                callback = self._subscribers[to_agent]
                if asyncio.iscoroutinefunction(callback):
                    await callback(message)
                else:
                    callback(message)
            except Exception as e:
                logger.error(f"Error delivering message to {to_agent}: {e}")
        else:
            logger.warning(f"Agent {to_agent} not found for message delivery")
    
    async def broadcast(self, message: AgentMessage) -> None:
        """Broadcast a message to all agents."""
        message.to_agent = "*"
        
        if message.is_expired():
            logger.warning(f"Message {message.id} expired, not broadcasting")
            return
        
        # Deliver to all subscribers except sender
        for agent_id, callback in list(self._subscribers.items()):
            if agent_id == message.from_agent:
                continue
            
            try:
                if asyncio.iscoroutinefunction(callback):
                    await callback(message)
                else:
                    callback(message)
            except Exception as e:
                logger.error(f"Error broadcasting to {agent_id}: {e}")
    
    async def request(
        self,
        message: AgentMessage,
        timeout: float = 30.0,
    ) -> Optional[AgentMessage]:
        """Send request and wait for response."""
        # Set correlation ID if not set
        if not message.correlation_id:
            message.correlation_id = str(uuid.uuid4())
        
        # Create future for response
        future: asyncio.Future = asyncio.get_event_loop().create_future()
        self._pending_requests[message.correlation_id] = future
        
        try:
            # Send request
            await self.publish(message)
            
            # Wait for response
            response = await asyncio.wait_for(future, timeout=timeout)
            return response
        except asyncio.TimeoutError:
            logger.warning(f"Request {message.id} timed out after {timeout}s")
            return None
        finally:
            # Cleanup
            self._pending_requests.pop(message.correlation_id, None)
    
    async def get_agents(self) -> List[str]:
        """Get list of registered agents."""
        return list(self._subscribers.keys())
    
    async def publish_to_topic(self, topic: str, message: AgentMessage) -> None:
        """Publish a message to a topic."""
        if topic not in self._topic_subscribers:
            return
        
        for agent_id in self._topic_subscribers[topic]:
            if agent_id == message.from_agent:
                continue
            
            if agent_id in self._subscribers:
                try:
                    callback = self._subscribers[agent_id]
                    if asyncio.iscoroutinefunction(callback):
                        await callback(message)
                    else:
                        callback(message)
                except Exception as e:
                    logger.error(f"Error publishing to topic {topic}, agent {agent_id}: {e}")


# Singleton instance for local communication
_local_bus: Optional[LocalMessageBus] = None


def create_message_bus(
    bus_type: str = "local",
    **kwargs,
) -> MessageBus:
    """
    Create or get a message bus instance.
    
    Args:
        bus_type: "local" or "redis" (future)
        **kwargs: Bus-specific options
    
    Returns:
        MessageBus instance
    """
    global _local_bus
    
    if bus_type == "local":
        if _local_bus is None:
            _local_bus = LocalMessageBus()
        return _local_bus
    elif bus_type == "redis":
        # TODO: Implement RedisMessageBus
        logger.warning("Redis bus not yet implemented, using local")
        if _local_bus is None:
            _local_bus = LocalMessageBus()
        return _local_bus
    else:
        raise ValueError(f"Unknown bus type: {bus_type}")


# =============================================================================
# Helper functions for common message patterns
# =============================================================================

async def request_data(
    bus: MessageBus,
    from_agent: str,
    to_agent: str,
    data_type: str,
    params: Dict[str, Any],
    timeout: float = 30.0,
) -> Optional[Dict[str, Any]]:
    """
    Request data from another agent.
    
    Args:
        bus: Message bus to use
        from_agent: Requesting agent ID
        to_agent: Agent to request from
        data_type: Type of data requested
        params: Request parameters
        timeout: Response timeout
    
    Returns:
        Response payload or None
    """
    message = AgentMessage(
        from_agent=from_agent,
        to_agent=to_agent,
        message_type=MessageType.DATA_REQUEST,
        payload={
            "data_type": data_type,
            "params": params,
        },
    )
    
    response = await bus.request(message, timeout)
    
    if response:
        return response.payload
    return None


async def share_signal(
    bus: MessageBus,
    from_agent: str,
    signal: Dict[str, Any],
    to_agents: Optional[List[str]] = None,
) -> None:
    """
    Share a trading signal with other agents.
    
    Args:
        bus: Message bus to use
        from_agent: Sharing agent ID
        signal: Signal data
        to_agents: Specific agents to share with (None = broadcast)
    """
    message = AgentMessage(
        from_agent=from_agent,
        to_agent="*",
        message_type=MessageType.SIGNAL_SHARE,
        payload=signal,
    )
    
    if to_agents:
        for agent_id in to_agents:
            message.to_agent = agent_id
            await bus.publish(message)
    else:
        await bus.broadcast(message)


async def send_alert(
    bus: MessageBus,
    from_agent: str,
    alert_type: str,
    message_text: str,
    severity: str = "info",
    to_agents: Optional[List[str]] = None,
) -> None:
    """
    Send an alert to other agents.
    
    Args:
        bus: Message bus to use
        from_agent: Sending agent ID
        alert_type: Type of alert
        message_text: Alert message
        severity: "info", "warning", "error", "critical"
        to_agents: Specific agents (None = broadcast)
    """
    message = AgentMessage(
        from_agent=from_agent,
        to_agent="*",
        message_type=MessageType.ALERT,
        payload={
            "alert_type": alert_type,
            "message": message_text,
            "severity": severity,
        },
        priority=1 if severity == "critical" else 3 if severity == "error" else 5,
    )
    
    if to_agents:
        for agent_id in to_agents:
            message.to_agent = agent_id
            await bus.publish(message)
    else:
        await bus.broadcast(message)
