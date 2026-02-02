//! Agent Communication System
//!
//! High-performance message passing between AI trading agents.
//!
//! # Features
//!
//! - **Direct Messaging**: Point-to-point communication between agents
//! - **Pub/Sub**: Topic-based broadcast messaging
//! - **Request/Response**: Correlated request-response patterns
//! - **Priority Queues**: Message prioritization for time-sensitive data
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │   Agent A   │     │   Agent B   │     │   Agent C   │
//! └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
//!        │                   │                   │
//!        └───────────────────┼───────────────────┘
//!                            │
//!                   ┌────────▼────────┐
//!                   │   MessageBus    │
//!                   │  ┌───────────┐  │
//!                   │  │  Topics   │  │
//!                   │  │  Queues   │  │
//!                   │  │  Router   │  │
//!                   │  └───────────┘  │
//!                   └─────────────────┘
//! ```

pub mod bus;
pub mod error;
pub mod message;

pub use bus::{LocalMessageBus, MessageBus};
pub use error::{CommError, CommResult};
pub use message::{AgentMessage, MessagePriority, MessageType};
