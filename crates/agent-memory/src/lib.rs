//! Agent Memory System
//!
//! High-performance memory persistence for AI trading agents.
//! Provides persistent storage with semantic search capabilities.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    MemoryManager                        │
//! │  - Manages multiple memory stores                       │
//! │  - Handles memory lifecycle                             │
//! └─────────────────────────────────────────────────────────┘
//!                            │
//!          ┌─────────────────┼─────────────────┐
//!          ▼                 ▼                 ▼
//!   ┌────────────┐   ┌────────────┐   ┌────────────┐
//!   │ ShortTerm  │   │ LongTerm   │   │  Vector    │
//!   │  (Cache)   │   │ (SQLite)   │   │  (Search)  │
//!   └────────────┘   └────────────┘   └────────────┘
//! ```

pub mod entry;
pub mod error;
pub mod manager;
pub mod store;
pub mod vector;

pub use entry::{MemoryEntry, MemoryQuery, MemoryType};
pub use error::{MemoryError, MemoryResult};
pub use manager::{MemoryConfig, MemoryManager};
pub use store::{MemoryStore, SqliteMemoryStore, InMemoryStore};
pub use vector::VectorIndex;
