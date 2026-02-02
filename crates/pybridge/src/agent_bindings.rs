//! Python bindings for AI Agent components.
//!
//! Exposes Rust memory, communication, and tools to Python.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;

// Re-exports from Rust crates
use agent_comm::{AgentMessage, LocalMessageBus, MessageBus, MessageType};
use agent_core::{AnalysisFormatter, MarketDataFormatter, PortfolioFormatter, SignalFormatter, ToolRegistry};
use agent_memory::{MemoryEntry, MemoryManager, MemoryType};

// =============================================================================
// Memory Types
// =============================================================================

/// Python wrapper for MemoryType enum.
#[pyclass(name = "MemoryType")]
#[derive(Clone)]
pub struct PyMemoryType(MemoryType);

#[pymethods]
impl PyMemoryType {
    #[staticmethod]
    fn observation() -> Self {
        Self(MemoryType::Observation)
    }

    #[staticmethod]
    fn decision() -> Self {
        Self(MemoryType::Decision)
    }

    #[staticmethod]
    fn action() -> Self {
        Self(MemoryType::Action)
    }

    #[staticmethod]
    fn outcome() -> Self {
        Self(MemoryType::Outcome)
    }

    #[staticmethod]
    fn learning() -> Self {
        Self(MemoryType::Learning)
    }

    #[staticmethod]
    fn context() -> Self {
        Self(MemoryType::Context)
    }

    #[staticmethod]
    fn conversation() -> Self {
        Self(MemoryType::Conversation)
    }

    fn __repr__(&self) -> String {
        format!("MemoryType::{:?}", self.0)
    }
}

/// Python wrapper for MemoryEntry.
#[pyclass(name = "MemoryEntry")]
pub struct PyMemoryEntry {
    inner: MemoryEntry,
}

#[pymethods]
impl PyMemoryEntry {
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    #[getter]
    fn agent_id(&self) -> String {
        self.inner.agent_id.clone()
    }

    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    #[getter]
    fn importance(&self) -> f64 {
        self.inner.importance
    }

    #[getter]
    fn created_at(&self) -> String {
        self.inner.created_at.to_rfc3339()
    }

    #[getter]
    fn access_count(&self) -> u32 {
        self.inner.access_count
    }

    fn relevance_score(&self) -> f64 {
        self.inner.relevance_score()
    }

    fn is_expired(&self) -> bool {
        self.inner.is_expired()
    }

    fn __repr__(&self) -> String {
        format!(
            "MemoryEntry(id={}, content='{}...')",
            self.inner.id,
            &self.inner.content[..self.inner.content.len().min(50)]
        )
    }
}

/// Python wrapper for MemoryManager.
#[pyclass(name = "MemoryManager")]
pub struct PyMemoryManager {
    inner: Arc<MemoryManager>,
    runtime: Arc<tokio::runtime::Runtime>,
}

#[pymethods]
impl PyMemoryManager {
    #[new]
    #[pyo3(signature = (db_path=None))]
    fn new(db_path: Option<String>) -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        let config = agent_memory::MemoryConfig {
            db_path,
            ..Default::default()
        };
        
        let manager = MemoryManager::new(config)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        Ok(Self {
            inner: Arc::new(manager),
            runtime: Arc::new(runtime),
        })
    }

    /// Store a memory.
    #[pyo3(signature = (agent_id, content, memory_type, importance=None, metadata=None))]
    fn remember(
        &self,
        agent_id: &str,
        content: &str,
        memory_type: &PyMemoryType,
        importance: Option<f64>,
        metadata: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<String> {
        let meta = metadata
            .map(|d| {
                let json_str = pythonize_to_json(d)?;
                serde_json::from_str(&json_str).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
                })
            })
            .transpose()?;

        let manager = self.inner.clone();
        let agent_id = agent_id.to_string();
        let content = content.to_string();
        let mt = memory_type.0;

        let result = self.runtime.block_on(async move {
            manager
                .remember(&agent_id, content, mt, importance, meta, None)
                .await
        });

        result
            .map(|id| id.to_string())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Recall memories.
    #[pyo3(signature = (agent_id, query=None, memory_type=None, limit=10))]
    fn recall(
        &self,
        agent_id: &str,
        query: Option<&str>,
        memory_type: Option<&PyMemoryType>,
        limit: usize,
    ) -> PyResult<Vec<PyMemoryEntry>> {
        let manager = self.inner.clone();
        let agent_id = agent_id.to_string();
        let query = query.map(|s| s.to_string());
        let mt = memory_type.map(|m| m.0);

        let result = self.runtime.block_on(async move {
            manager
                .recall(&agent_id, query.as_deref(), mt, limit, None)
                .await
        });

        result
            .map(|entries| {
                entries
                    .into_iter()
                    .map(|e| PyMemoryEntry { inner: e })
                    .collect()
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get memory count for an agent.
    fn count(&self, agent_id: &str) -> PyResult<usize> {
        let manager = self.inner.clone();
        let agent_id = agent_id.to_string();

        let result = self.runtime.block_on(async move { manager.count(&agent_id).await });

        result.map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Clear all memories for an agent.
    fn clear(&self, agent_id: &str) -> PyResult<usize> {
        let manager = self.inner.clone();
        let agent_id = agent_id.to_string();

        let result = self.runtime.block_on(async move { manager.clear_agent(&agent_id).await });

        result.map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
}

// =============================================================================
// Communication Types
// =============================================================================

/// Python wrapper for MessageType enum.
#[pyclass(name = "MessageType")]
#[derive(Clone)]
pub struct PyMessageType(MessageType);

#[pymethods]
impl PyMessageType {
    #[staticmethod]
    fn data_request() -> Self {
        Self(MessageType::DataRequest)
    }

    #[staticmethod]
    fn data_response() -> Self {
        Self(MessageType::DataResponse)
    }

    #[staticmethod]
    fn signal_share() -> Self {
        Self(MessageType::SignalShare)
    }

    #[staticmethod]
    fn alert() -> Self {
        Self(MessageType::Alert)
    }

    #[staticmethod]
    fn status() -> Self {
        Self(MessageType::Status)
    }

    fn __repr__(&self) -> String {
        format!("MessageType::{:?}", self.0)
    }
}

/// Python wrapper for LocalMessageBus.
#[pyclass(name = "MessageBus")]
pub struct PyMessageBus {
    inner: Arc<LocalMessageBus>,
    runtime: Arc<tokio::runtime::Runtime>,
}

#[pymethods]
impl PyMessageBus {
    #[new]
    fn new() -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(LocalMessageBus::new()),
            runtime: Arc::new(runtime),
        })
    }

    /// Register an agent.
    fn register(&self, agent_id: &str) -> PyResult<()> {
        let bus = self.inner.clone();
        let agent_id = agent_id.to_string();

        self.runtime.block_on(async move { bus.register(&agent_id).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Unregister an agent.
    fn unregister(&self, agent_id: &str) -> PyResult<()> {
        let bus = self.inner.clone();
        let agent_id = agent_id.to_string();

        self.runtime.block_on(async move { bus.unregister(&agent_id).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Send a direct message.
    fn send_direct(
        &self,
        from_agent: &str,
        to_agent: &str,
        message_type: &PyMessageType,
        payload: &Bound<'_, PyDict>,
    ) -> PyResult<()> {
        let json_str = pythonize_to_json(payload)?;
        let payload: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let msg = AgentMessage::direct(from_agent, to_agent, message_type.0, payload);
        let bus = self.inner.clone();

        self.runtime.block_on(async move { bus.send(msg).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Broadcast to a topic.
    fn broadcast(
        &self,
        from_agent: &str,
        topic: &str,
        message_type: &PyMessageType,
        payload: &Bound<'_, PyDict>,
    ) -> PyResult<()> {
        let json_str = pythonize_to_json(payload)?;
        let payload: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let msg = AgentMessage::broadcast(from_agent, topic, message_type.0, payload);
        let bus = self.inner.clone();

        self.runtime.block_on(async move { bus.send(msg).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Subscribe to a topic.
    fn subscribe(&self, agent_id: &str, topic: &str) -> PyResult<String> {
        let bus = self.inner.clone();
        let agent_id = agent_id.to_string();
        let topic = topic.to_string();

        let result = self.runtime.block_on(async move { bus.subscribe(&agent_id, &topic).await });

        result
            .map(|sub| sub.id.to_string())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get pending message count.
    fn pending_count(&self, agent_id: &str) -> PyResult<usize> {
        let bus = self.inner.clone();
        let agent_id = agent_id.to_string();

        self.runtime.block_on(async move { bus.pending_count(&agent_id).await })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
}

// =============================================================================
// Formatters
// =============================================================================

/// Python wrapper for MarketDataFormatter.
#[pyclass(name = "MarketDataFormatter")]
pub struct PyMarketDataFormatter;

#[pymethods]
impl PyMarketDataFormatter {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Format ticker data as text.
    #[staticmethod]
    fn format_ticker(data: &Bound<'_, PyDict>) -> PyResult<String> {
        let json_str = pythonize_to_json(data)?;
        let value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(MarketDataFormatter::format_ticker(&value))
    }

    /// Format orderbook as text.
    #[staticmethod]
    fn format_orderbook(data: &Bound<'_, PyDict>) -> PyResult<String> {
        let json_str = pythonize_to_json(data)?;
        let value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(MarketDataFormatter::format_orderbook(&value))
    }

    /// Format candles as text.
    #[staticmethod]
    #[pyo3(signature = (data, limit=10))]
    fn format_candles(data: &Bound<'_, PyDict>, limit: usize) -> PyResult<String> {
        let json_str = pythonize_to_json(data)?;
        let value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(MarketDataFormatter::format_candles(&value, limit))
    }
}

/// Python wrapper for SignalFormatter.
#[pyclass(name = "SignalFormatter")]
pub struct PySignalFormatter;

#[pymethods]
impl PySignalFormatter {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Format a signal as text.
    #[staticmethod]
    fn format_signal(data: &Bound<'_, PyDict>) -> PyResult<String> {
        let json_str = pythonize_to_json(data)?;
        let value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(SignalFormatter::format_signal(&value))
    }
}

/// Python wrapper for PortfolioFormatter.
#[pyclass(name = "PortfolioFormatter")]
pub struct PyPortfolioFormatter;

#[pymethods]
impl PyPortfolioFormatter {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Format portfolio as text.
    #[staticmethod]
    fn format_portfolio(data: &Bound<'_, PyDict>) -> PyResult<String> {
        let json_str = pythonize_to_json(data)?;
        let value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PortfolioFormatter::format_portfolio(&value))
    }
}

/// Python wrapper for AnalysisFormatter.
#[pyclass(name = "AnalysisFormatter")]
pub struct PyAnalysisFormatter;

#[pymethods]
impl PyAnalysisFormatter {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Format full analysis as text.
    #[staticmethod]
    fn format_analysis(data: &Bound<'_, PyDict>) -> PyResult<String> {
        let json_str = pythonize_to_json(data)?;
        let value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(AnalysisFormatter::format_full_analysis(&value))
    }
}

// =============================================================================
// Tool Registry
// =============================================================================

/// Python wrapper for ToolRegistry.
#[pyclass(name = "ToolRegistry")]
pub struct PyToolRegistry {
    inner: ToolRegistry,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PyToolRegistry {
    #[new]
    fn new() -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(Self {
            inner: ToolRegistry::with_defaults(),
            runtime,
        })
    }

    /// List available tools.
    fn list_tools(&self) -> Vec<String> {
        self.inner.list()
    }

    /// Execute a tool.
    fn execute(&self, tool_name: &str, params: &Bound<'_, PyDict>) -> PyResult<PyObject> {
        let json_str = pythonize_to_json(params)?;
        let params: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        let tool_name = tool_name.to_string();
        let registry = &self.inner;

        let result = self.runtime.block_on(async { registry.execute(&tool_name, params).await });

        let tool_result = result
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Python::with_gil(|py| {
            let dict = PyDict::new_bound(py);
            dict.set_item("success", tool_result.success)?;
            dict.set_item("execution_time_ms", tool_result.execution_time_ms)?;

            if let Some(output) = tool_result.output {
                let output_str = serde_json::to_string(&output).unwrap_or_default();
                dict.set_item("output", output_str)?;
            }

            if let Some(error) = tool_result.error {
                dict.set_item("error", error)?;
            }

            Ok(dict.into())
        })
    }

    /// Get OpenAI function schemas.
    fn openai_schemas(&self) -> PyResult<String> {
        let schemas = self.inner.openai_schemas();
        serde_json::to_string(&schemas)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get Anthropic tool schemas.
    fn anthropic_schemas(&self) -> PyResult<String> {
        let schemas = self.inner.anthropic_schemas();
        serde_json::to_string(&schemas)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
}

// =============================================================================
// Helpers
// =============================================================================

/// Convert Python dict to JSON string.
fn pythonize_to_json(dict: &Bound<'_, PyDict>) -> PyResult<String> {
    Python::with_gil(|py| {
        let json_module = py.import_bound("json")?;
        let json_str: String = json_module
            .call_method1("dumps", (dict,))?
            .extract()?;
        Ok(json_str)
    })
}

// =============================================================================
// Module Registration
// =============================================================================

/// Register agent-related types with the Python module.
pub fn register_agent_types(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Memory types
    m.add_class::<PyMemoryType>()?;
    m.add_class::<PyMemoryEntry>()?;
    m.add_class::<PyMemoryManager>()?;

    // Communication types
    m.add_class::<PyMessageType>()?;
    m.add_class::<PyMessageBus>()?;

    // Formatters
    m.add_class::<PyMarketDataFormatter>()?;
    m.add_class::<PySignalFormatter>()?;
    m.add_class::<PyPortfolioFormatter>()?;
    m.add_class::<PyAnalysisFormatter>()?;

    // Tool registry
    m.add_class::<PyToolRegistry>()?;

    Ok(())
}
