"""
Memory System for AI Agents

Provides persistent memory storage with:
- Short-term memory (session-based, in-memory)
- Long-term memory (persistent, database-backed)
- Vector memory (semantic search, embeddings)

Memory is crucial for AI trading agents to:
- Learn from past decisions
- Maintain context across sessions
- Recall relevant market patterns
"""

from __future__ import annotations

import json
import logging
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Union
import hashlib

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Module-level constants
# ---------------------------------------------------------------------------
DEFAULT_SHORT_TERM_LIMIT = 100


class MemoryType(str, Enum):
    """Types of memories an agent can store."""
    OBSERVATION = "observation"  # Market observations, data points
    DECISION = "decision"        # Trading decisions made
    ACTION = "action"            # Actions taken (tool calls, orders)
    OUTCOME = "outcome"          # Results of actions
    LEARNING = "learning"        # Insights derived from experience
    CONTEXT = "context"          # Session context, state
    CONVERSATION = "conversation"  # Agent-to-agent communication


@dataclass
class MemoryEntry:
    """A single memory entry."""
    memory_type: MemoryType
    content: str
    metadata: Dict[str, Any] = field(default_factory=dict)
    timestamp: datetime = field(default_factory=datetime.now)
    id: Optional[str] = None
    embedding: Optional[List[float]] = None
    importance: float = 0.5  # 0-1 scale
    
    def __post_init__(self):
        if self.id is None:
            # Generate deterministic ID from content
            content_hash = hashlib.sha256(
                f"{self.timestamp.isoformat()}{self.content}".encode()
            ).hexdigest()[:16]
            self.id = f"mem_{content_hash}"
    
    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {
            "id": self.id,
            "memory_type": self.memory_type.value,
            "content": self.content,
            "metadata": self.metadata,
            "timestamp": self.timestamp.isoformat(),
            "importance": self.importance,
        }
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "MemoryEntry":
        """Create from dictionary."""
        return cls(
            id=data.get("id"),
            memory_type=MemoryType(data["memory_type"]),
            content=data["content"],
            metadata=data.get("metadata", {}),
            timestamp=datetime.fromisoformat(data["timestamp"]),
            importance=data.get("importance", 0.5),
        )


class MemoryStore(ABC):
    """
    Abstract base class for memory storage backends.
    
    Implementations:
    - LocalMemoryStore: In-memory storage (for development)
    - RedisMemoryStore: Redis-backed (for short-term, distributed)
    - PostgresMemoryStore: PostgreSQL-backed (for long-term persistence)
    - VectorMemoryStore: Vector DB with embeddings (for semantic search)
    """
    
    @abstractmethod
    async def store(self, entry: MemoryEntry) -> str:
        """
        Store a memory entry.
        
        Returns:
            Memory ID
        """
        pass
    
    @abstractmethod
    async def recall(
        self,
        query: str,
        limit: int = 10,
        memory_type: Optional[MemoryType] = None,
        since: Optional[datetime] = None,
        min_importance: float = 0.0,
    ) -> List[MemoryEntry]:
        """
        Recall memories matching a query.
        
        Args:
            query: Search query (semantic or keyword)
            limit: Maximum number of memories to return
            memory_type: Filter by memory type
            since: Only return memories after this time
            min_importance: Minimum importance threshold
        
        Returns:
            List of matching memories, ordered by relevance
        """
        pass
    
    @abstractmethod
    async def get(self, memory_id: str) -> Optional[MemoryEntry]:
        """Get a specific memory by ID."""
        pass
    
    @abstractmethod
    async def delete(self, memory_id: str) -> bool:
        """Delete a memory by ID."""
        pass
    
    @abstractmethod
    async def clear(self, memory_type: Optional[MemoryType] = None) -> int:
        """
        Clear memories.
        
        Args:
            memory_type: If provided, only clear this type
        
        Returns:
            Number of memories cleared
        """
        pass
    
    async def count(self, memory_type: Optional[MemoryType] = None) -> int:
        """Count stored memories."""
        return 0
    
    async def get_recent(
        self,
        limit: int = 10,
        memory_type: Optional[MemoryType] = None,
    ) -> List[MemoryEntry]:
        """Get most recent memories."""
        return await self.recall("", limit=limit, memory_type=memory_type)


class LocalMemoryStore(MemoryStore):
    """
    In-memory storage for development and testing.
    
    Features:
    - Fast access
    - Keyword-based search
    - No persistence (lost on restart)
    """
    
    def __init__(self, persist_path: Optional[Path] = None):
        self._memories: Dict[str, MemoryEntry] = {}
        self._persist_path = persist_path
        
        # Load persisted memories if path provided
        if persist_path and persist_path.exists():
            self._load_from_file()
    
    async def store(self, entry: MemoryEntry) -> str:
        """Store a memory entry."""
        self._memories[entry.id] = entry
        
        # Persist if path configured
        if self._persist_path:
            self._save_to_file()
        
        logger.debug(f"Stored memory {entry.id}: {entry.memory_type.value}")
        return entry.id
    
    async def recall(
        self,
        query: str,
        limit: int = 10,
        memory_type: Optional[MemoryType] = None,
        since: Optional[datetime] = None,
        min_importance: float = 0.0,
    ) -> List[MemoryEntry]:
        """Recall memories using keyword matching."""
        results = []
        query_lower = query.lower()
        
        for memory in self._memories.values():
            # Filter by type
            if memory_type and memory.memory_type != memory_type:
                continue
            
            # Filter by time
            if since and memory.timestamp < since:
                continue
            
            # Filter by importance
            if memory.importance < min_importance:
                continue
            
            # Score by keyword match
            if query:
                content_lower = memory.content.lower()
                if query_lower in content_lower:
                    # Calculate simple relevance score
                    score = content_lower.count(query_lower) / len(content_lower)
                    results.append((score, memory))
            else:
                results.append((memory.importance, memory))
        
        # Sort by score descending
        results.sort(key=lambda x: x[0], reverse=True)
        
        return [m for _, m in results[:limit]]
    
    async def get(self, memory_id: str) -> Optional[MemoryEntry]:
        """Get a specific memory."""
        return self._memories.get(memory_id)
    
    async def delete(self, memory_id: str) -> bool:
        """Delete a memory."""
        if memory_id in self._memories:
            del self._memories[memory_id]
            if self._persist_path:
                self._save_to_file()
            return True
        return False
    
    async def clear(self, memory_type: Optional[MemoryType] = None) -> int:
        """Clear memories."""
        if memory_type is None:
            count = len(self._memories)
            self._memories.clear()
        else:
            to_delete = [
                mid for mid, m in self._memories.items()
                if m.memory_type == memory_type
            ]
            count = len(to_delete)
            for mid in to_delete:
                del self._memories[mid]
        
        if self._persist_path:
            self._save_to_file()
        
        return count
    
    async def count(self, memory_type: Optional[MemoryType] = None) -> int:
        """Count memories."""
        if memory_type is None:
            return len(self._memories)
        return sum(1 for m in self._memories.values() if m.memory_type == memory_type)
    
    def _save_to_file(self) -> None:
        """Persist memories to file."""
        if not self._persist_path:
            return
        
        self._persist_path.parent.mkdir(parents=True, exist_ok=True)
        
        data = {
            mid: entry.to_dict()
            for mid, entry in self._memories.items()
        }
        
        with open(self._persist_path, "w") as f:
            json.dump(data, f, indent=2)
    
    def _load_from_file(self) -> None:
        """Load memories from file."""
        if not self._persist_path or not self._persist_path.exists():
            return
        
        try:
            with open(self._persist_path, "r") as f:
                data = json.load(f)
            
            self._memories = {
                mid: MemoryEntry.from_dict(entry_data)
                for mid, entry_data in data.items()
            }
            logger.info(f"Loaded {len(self._memories)} memories from {self._persist_path}")
        except Exception as e:
            logger.error(f"Failed to load memories: {e}")


class CompositeMemoryStore(MemoryStore):
    """
    Composite memory store combining multiple backends.
    
    Typical setup:
    - Short-term: LocalMemoryStore (recent context)
    - Long-term: PostgresMemoryStore (persistent history)
    - Vector: VectorMemoryStore (semantic search)
    """
    
    def __init__(
        self,
        short_term: MemoryStore,
        long_term: Optional[MemoryStore] = None,
        vector: Optional[MemoryStore] = None,
        short_term_limit: int = DEFAULT_SHORT_TERM_LIMIT,
    ):
        self._short_term = short_term
        self._long_term = long_term
        self._vector = vector
        self._short_term_limit = short_term_limit
    
    async def store(self, entry: MemoryEntry) -> str:
        """Store in all configured backends."""
        # Always store in short-term
        memory_id = await self._short_term.store(entry)
        
        # Store important memories in long-term
        if self._long_term and entry.importance >= 0.5:
            await self._long_term.store(entry)
        
        # Store in vector store for semantic search
        if self._vector:
            await self._vector.store(entry)
        
        # Prune short-term if needed
        count = await self._short_term.count()
        if count > self._short_term_limit:
            await self._prune_short_term()
        
        return memory_id
    
    async def recall(
        self,
        query: str,
        limit: int = 10,
        memory_type: Optional[MemoryType] = None,
        since: Optional[datetime] = None,
        min_importance: float = 0.0,
    ) -> List[MemoryEntry]:
        """
        Recall from multiple sources and merge results.
        
        Priority:
        1. Vector search (if available) for semantic matches
        2. Short-term for recent context
        3. Long-term for historical patterns
        """
        results = []
        seen_ids = set()
        
        # Vector search first (most relevant)
        if self._vector:
            vector_results = await self._vector.recall(
                query, limit=limit, memory_type=memory_type,
                since=since, min_importance=min_importance
            )
            for m in vector_results:
                if m.id not in seen_ids:
                    results.append(m)
                    seen_ids.add(m.id)
        
        # Short-term for recent
        short_results = await self._short_term.recall(
            query, limit=limit, memory_type=memory_type,
            since=since, min_importance=min_importance
        )
        for m in short_results:
            if m.id not in seen_ids:
                results.append(m)
                seen_ids.add(m.id)
        
        # Long-term for history
        if self._long_term and len(results) < limit:
            long_results = await self._long_term.recall(
                query, limit=limit - len(results), memory_type=memory_type,
                since=since, min_importance=min_importance
            )
            for m in long_results:
                if m.id not in seen_ids:
                    results.append(m)
                    seen_ids.add(m.id)
        
        return results[:limit]
    
    async def get(self, memory_id: str) -> Optional[MemoryEntry]:
        """Get from any backend."""
        result = await self._short_term.get(memory_id)
        if result:
            return result
        
        if self._long_term:
            result = await self._long_term.get(memory_id)
            if result:
                return result
        
        if self._vector:
            result = await self._vector.get(memory_id)
            if result:
                return result
        
        return None
    
    async def delete(self, memory_id: str) -> bool:
        """Delete from all backends."""
        deleted = await self._short_term.delete(memory_id)
        
        if self._long_term:
            deleted = await self._long_term.delete(memory_id) or deleted
        
        if self._vector:
            deleted = await self._vector.delete(memory_id) or deleted
        
        return deleted
    
    async def clear(self, memory_type: Optional[MemoryType] = None) -> int:
        """Clear all backends."""
        count = await self._short_term.clear(memory_type)
        
        if self._long_term:
            count += await self._long_term.clear(memory_type)
        
        if self._vector:
            count += await self._vector.clear(memory_type)
        
        return count
    
    async def _prune_short_term(self) -> None:
        """Prune oldest short-term memories, moving important ones to long-term."""
        # Get all short-term memories
        all_memories = await self._short_term.recall("", limit=self._short_term_limit * 2)
        
        # Sort by timestamp
        all_memories.sort(key=lambda m: m.timestamp)
        
        # Keep newest, archive important old ones
        to_archive = all_memories[:-self._short_term_limit]
        
        for memory in to_archive:
            if self._long_term and memory.importance >= 0.3:
                await self._long_term.store(memory)
            await self._short_term.delete(memory.id)


def create_memory_store(
    backend: str = "local",
    vector_store: Optional[str] = None,
    persist_path: Optional[str] = None,
    **kwargs,
) -> MemoryStore:
    """
    Factory function to create a memory store.
    
    Args:
        backend: "local", "redis", or "postgres"
        vector_store: Optional vector store: "chromadb", "pinecone"
        persist_path: Path for local persistence
        **kwargs: Backend-specific options
    
    Returns:
        Configured MemoryStore instance
    """
    # Create primary store
    if backend == "local":
        path = Path(persist_path) if persist_path else None
        primary = LocalMemoryStore(persist_path=path)
    elif backend == "redis":
        # TODO: Implement RedisMemoryStore
        logger.warning("Redis backend not yet implemented, using local")
        path = Path(persist_path) if persist_path else None
        primary = LocalMemoryStore(persist_path=path)
    elif backend == "postgres":
        # TODO: Implement PostgresMemoryStore
        logger.warning("Postgres backend not yet implemented, using local")
        path = Path(persist_path) if persist_path else None
        primary = LocalMemoryStore(persist_path=path)
    else:
        raise ValueError(f"Unknown backend: {backend}")
    
    # Create vector store if requested
    vector = None
    if vector_store:
        # TODO: Implement vector stores
        logger.warning(f"Vector store '{vector_store}' not yet implemented")
    
    # Return composite if we have multiple stores
    if vector:
        return CompositeMemoryStore(
            short_term=primary,
            vector=vector,
        )
    
    return primary


__all__ = [
    "DEFAULT_SHORT_TERM_LIMIT",
    "MemoryType",
    "MemoryEntry",
    "MemoryStore",
    "LocalMemoryStore",
    "CompositeMemoryStore",
    "create_memory_store",
]
