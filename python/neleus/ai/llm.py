"""
LLM Provider Abstraction

Provider-agnostic interface for LLM integration:
- OpenAI (GPT-4, GPT-4o)
- Anthropic (Claude)
- Local models (Ollama, vLLM)

Supports:
- Chat completions
- Tool/function calling
- Streaming responses
"""

from __future__ import annotations

import logging
import os
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from typing import Any, Dict, List, Optional, AsyncIterator, Union

logger = logging.getLogger(__name__)


class Role(str, Enum):
    """Message roles."""
    SYSTEM = "system"
    USER = "user"
    ASSISTANT = "assistant"
    TOOL = "tool"


@dataclass
class Message:
    """A chat message."""
    role: str  # "system", "user", "assistant", "tool"
    content: str
    name: Optional[str] = None  # For tool messages
    tool_calls: Optional[List[Dict[str, Any]]] = None  # For assistant tool calls
    tool_call_id: Optional[str] = None  # For tool responses
    
    def to_openai(self) -> Dict[str, Any]:
        """Convert to OpenAI format."""
        msg = {"role": self.role, "content": self.content}
        if self.name:
            msg["name"] = self.name
        if self.tool_calls:
            msg["tool_calls"] = self.tool_calls
        if self.tool_call_id:
            msg["tool_call_id"] = self.tool_call_id
        return msg
    
    def to_anthropic(self) -> Dict[str, Any]:
        """Convert to Anthropic format."""
        if self.role == "system":
            # Anthropic handles system messages differently
            return None
        return {"role": self.role, "content": self.content}


@dataclass
class ToolCall:
    """A tool call from the LLM."""
    id: str
    name: str
    arguments: Dict[str, Any]


@dataclass
class CompletionResult:
    """Result of an LLM completion."""
    content: str
    tool_calls: List[ToolCall] = field(default_factory=list)
    finish_reason: str = "stop"  # "stop", "tool_calls", "length"
    usage: Dict[str, int] = field(default_factory=dict)  # tokens used
    model: str = ""
    
    @property
    def has_tool_calls(self) -> bool:
        """Check if response contains tool calls."""
        return len(self.tool_calls) > 0


class LLMProvider(ABC):
    """
    Abstract base class for LLM providers.
    
    Implementations should handle:
    - Chat completions
    - Tool/function calling
    - Streaming (optional)
    - Error handling and retries
    """
    
    def __init__(
        self,
        model: str,
        temperature: float = 0.7,
        max_tokens: int = 4096,
        **kwargs,
    ):
        self.model = model
        self.temperature = temperature
        self.max_tokens = max_tokens
        self.kwargs = kwargs
    
    @abstractmethod
    async def complete(
        self,
        messages: List[Message],
        tools: Optional[List[Any]] = None,
        tool_choice: Optional[str] = None,  # "auto", "none", or specific tool name
        **kwargs,
    ) -> CompletionResult:
        """
        Generate a completion.
        
        Args:
            messages: List of conversation messages
            tools: Optional list of Tool objects
            tool_choice: How to handle tool calls
            **kwargs: Provider-specific options
        
        Returns:
            CompletionResult with content and optional tool calls
        """
        pass
    
    async def stream(
        self,
        messages: List[Message],
        tools: Optional[List[Any]] = None,
        **kwargs,
    ) -> AsyncIterator[str]:
        """
        Stream a completion (optional, falls back to complete).
        
        Yields:
            Content chunks as they arrive
        """
        result = await self.complete(messages, tools, **kwargs)
        yield result.content


class OpenAIProvider(LLMProvider):
    """OpenAI API provider (GPT-4, GPT-4o, etc.)"""
    
    def __init__(
        self,
        model: str = "gpt-4o",
        temperature: float = 0.7,
        max_tokens: int = 4096,
        api_key: Optional[str] = None,
        **kwargs,
    ):
        super().__init__(model, temperature, max_tokens, **kwargs)
        self.api_key = api_key or os.environ.get("OPENAI_API_KEY")
        self._client = None
    
    def _get_client(self):
        """Get or create OpenAI client."""
        if self._client is None:
            try:
                from openai import AsyncOpenAI
                self._client = AsyncOpenAI(api_key=self.api_key)
            except ImportError:
                raise ImportError("openai package required: pip install openai")
        return self._client
    
    async def complete(
        self,
        messages: List[Message],
        tools: Optional[List[Any]] = None,
        tool_choice: Optional[str] = None,
        **kwargs,
    ) -> CompletionResult:
        """Generate completion using OpenAI API."""
        client = self._get_client()
        
        # Convert messages
        openai_messages = [m.to_openai() for m in messages]
        
        # Build request
        request = {
            "model": self.model,
            "messages": openai_messages,
            "temperature": self.temperature,
            "max_tokens": self.max_tokens,
        }
        
        # Add tools if provided
        if tools:
            openai_tools = []
            for tool in tools:
                if hasattr(tool, "to_openai_schema"):
                    openai_tools.append(tool.to_openai_schema())
                else:
                    openai_tools.append(tool)
            request["tools"] = openai_tools
            
            if tool_choice:
                if tool_choice in ("auto", "none"):
                    request["tool_choice"] = tool_choice
                else:
                    request["tool_choice"] = {"type": "function", "function": {"name": tool_choice}}
        
        # Make request
        try:
            response = await client.chat.completions.create(**request)
        except Exception as e:
            logger.error(f"OpenAI API error: {e}")
            raise
        
        # Parse response
        choice = response.choices[0]
        message = choice.message
        
        tool_calls = []
        if message.tool_calls:
            import json
            for tc in message.tool_calls:
                tool_calls.append(ToolCall(
                    id=tc.id,
                    name=tc.function.name,
                    arguments=json.loads(tc.function.arguments),
                ))
        
        return CompletionResult(
            content=message.content or "",
            tool_calls=tool_calls,
            finish_reason=choice.finish_reason,
            usage={
                "prompt_tokens": response.usage.prompt_tokens,
                "completion_tokens": response.usage.completion_tokens,
                "total_tokens": response.usage.total_tokens,
            },
            model=response.model,
        )
    
    async def stream(
        self,
        messages: List[Message],
        tools: Optional[List[Any]] = None,
        **kwargs,
    ) -> AsyncIterator[str]:
        """Stream completion using OpenAI API."""
        client = self._get_client()
        
        openai_messages = [m.to_openai() for m in messages]
        
        request = {
            "model": self.model,
            "messages": openai_messages,
            "temperature": self.temperature,
            "max_tokens": self.max_tokens,
            "stream": True,
        }
        
        try:
            stream = await client.chat.completions.create(**request)
            async for chunk in stream:
                if chunk.choices[0].delta.content:
                    yield chunk.choices[0].delta.content
        except Exception as e:
            logger.error(f"OpenAI streaming error: {e}")
            raise


class AnthropicProvider(LLMProvider):
    """Anthropic API provider (Claude models)"""
    
    def __init__(
        self,
        model: str = "claude-3-5-sonnet-20241022",
        temperature: float = 0.7,
        max_tokens: int = 4096,
        api_key: Optional[str] = None,
        **kwargs,
    ):
        super().__init__(model, temperature, max_tokens, **kwargs)
        self.api_key = api_key or os.environ.get("ANTHROPIC_API_KEY")
        self._client = None
    
    def _get_client(self):
        """Get or create Anthropic client."""
        if self._client is None:
            try:
                from anthropic import AsyncAnthropic
                self._client = AsyncAnthropic(api_key=self.api_key)
            except ImportError:
                raise ImportError("anthropic package required: pip install anthropic")
        return self._client
    
    async def complete(
        self,
        messages: List[Message],
        tools: Optional[List[Any]] = None,
        tool_choice: Optional[str] = None,
        **kwargs,
    ) -> CompletionResult:
        """Generate completion using Anthropic API."""
        client = self._get_client()
        
        # Extract system message
        system = ""
        anthropic_messages = []
        for m in messages:
            if m.role == "system":
                system = m.content
            else:
                anthropic_messages.append({"role": m.role, "content": m.content})
        
        # Build request
        request = {
            "model": self.model,
            "messages": anthropic_messages,
            "max_tokens": self.max_tokens,
        }
        
        if system:
            request["system"] = system
        
        # Add tools if provided
        if tools:
            anthropic_tools = []
            for tool in tools:
                if hasattr(tool, "to_anthropic_schema"):
                    anthropic_tools.append(tool.to_anthropic_schema())
                else:
                    anthropic_tools.append(tool)
            request["tools"] = anthropic_tools
        
        # Make request
        try:
            response = await client.messages.create(**request)
        except Exception as e:
            logger.error(f"Anthropic API error: {e}")
            raise
        
        # Parse response
        content = ""
        tool_calls = []
        
        for block in response.content:
            if block.type == "text":
                content = block.text
            elif block.type == "tool_use":
                tool_calls.append(ToolCall(
                    id=block.id,
                    name=block.name,
                    arguments=block.input,
                ))
        
        return CompletionResult(
            content=content,
            tool_calls=tool_calls,
            finish_reason=response.stop_reason or "stop",
            usage={
                "prompt_tokens": response.usage.input_tokens,
                "completion_tokens": response.usage.output_tokens,
                "total_tokens": response.usage.input_tokens + response.usage.output_tokens,
            },
            model=self.model,
        )


class OllamaProvider(LLMProvider):
    """Ollama local model provider"""
    
    def __init__(
        self,
        model: str = "llama3.2",
        temperature: float = 0.7,
        max_tokens: int = 4096,
        base_url: str = "http://localhost:11434",
        **kwargs,
    ):
        super().__init__(model, temperature, max_tokens, **kwargs)
        self.base_url = base_url
    
    async def complete(
        self,
        messages: List[Message],
        tools: Optional[List[Any]] = None,
        tool_choice: Optional[str] = None,
        **kwargs,
    ) -> CompletionResult:
        """Generate completion using Ollama API."""
        import aiohttp
        
        # Convert messages to Ollama format
        ollama_messages = [
            {"role": m.role, "content": m.content}
            for m in messages
        ]
        
        request = {
            "model": self.model,
            "messages": ollama_messages,
            "options": {
                "temperature": self.temperature,
                "num_predict": self.max_tokens,
            },
            "stream": False,
        }
        
        try:
            async with aiohttp.ClientSession() as session:
                async with session.post(
                    f"{self.base_url}/api/chat",
                    json=request,
                ) as resp:
                    data = await resp.json()
        except Exception as e:
            logger.error(f"Ollama API error: {e}")
            raise
        
        return CompletionResult(
            content=data.get("message", {}).get("content", ""),
            tool_calls=[],
            finish_reason="stop",
            usage={
                "prompt_tokens": data.get("prompt_eval_count", 0),
                "completion_tokens": data.get("eval_count", 0),
                "total_tokens": data.get("prompt_eval_count", 0) + data.get("eval_count", 0),
            },
            model=self.model,
        )


# =============================================================================
# Factory
# =============================================================================

def create_provider(
    provider: str = "openai",
    model: Optional[str] = None,
    temperature: float = 0.7,
    max_tokens: int = 4096,
    **kwargs,
) -> LLMProvider:
    """
    Create an LLM provider.
    
    Args:
        provider: Provider name ("openai", "anthropic", "ollama")
        model: Model name (provider-specific)
        temperature: Sampling temperature
        max_tokens: Maximum output tokens
        **kwargs: Provider-specific options
    
    Returns:
        Configured LLMProvider instance
    """
    providers = {
        "openai": (OpenAIProvider, "gpt-4o"),
        "anthropic": (AnthropicProvider, "claude-3-5-sonnet-20241022"),
        "ollama": (OllamaProvider, "llama3.2"),
    }
    
    if provider not in providers:
        raise ValueError(f"Unknown provider: {provider}. Available: {list(providers.keys())}")
    
    provider_class, default_model = providers[provider]
    
    return provider_class(
        model=model or default_model,
        temperature=temperature,
        max_tokens=max_tokens,
        **kwargs,
    )
