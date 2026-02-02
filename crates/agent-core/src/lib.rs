//! Agent Core - Types and Utilities for AI Trading Agents
//!
//! This crate provides core functionality for building AI trading agents:
//!
//! - **Formatters**: Convert trading data to LLM-friendly formats
//! - **Tools**: Define and execute agent capabilities
//! - **Config**: Agent configuration types
//!
//! # Architecture
//!
//! ```text
//! Python (thin client)
//!         │
//!         ▼
//! ┌─────────────────────────────────────────────────┐
//! │              agent-core (Rust)                  │
//! │  ┌─────────────┐  ┌───────────┐  ┌──────────┐  │
//! │  │  Formatters │  │   Tools   │  │  Config  │  │
//! │  └─────────────┘  └───────────┘  └──────────┘  │
//! └─────────────────────────────────────────────────┘
//!         │                   │
//!         ▼                   ▼
//! ┌────────────────┐  ┌────────────────┐
//! │  agent-memory  │  │   agent-comm   │
//! └────────────────┘  └────────────────┘
//! ```

pub mod config;
pub mod error;
pub mod formatters;
pub mod tools;

pub use config::{AgentConfig, InfoConfig, PersonalityConfig};
pub use error::{AgentError, AgentResult};
pub use formatters::{
    AnalysisFormatter, MarketDataFormatter, PortfolioFormatter, SignalFormatter,
};
pub use tools::{Tool, ToolParameter, ToolRegistry, ToolResult};
