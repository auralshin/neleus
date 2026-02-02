//! Tool definitions and execution framework.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{AgentError, AgentResult};

/// Parameter definition for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Parameter name
    pub name: String,
    /// Parameter description
    pub description: String,
    /// Parameter type: string, number, boolean, array, object
    pub param_type: String,
    /// Whether the parameter is required
    pub required: bool,
    /// Default value (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Enum values (if restricted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

impl ToolParameter {
    /// Create a required string parameter.
    pub fn required_string(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "string".to_string(),
            required: true,
            default: None,
            enum_values: None,
        }
    }

    /// Create an optional string parameter.
    pub fn optional_string(
        name: impl Into<String>,
        description: impl Into<String>,
        default: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "string".to_string(),
            required: false,
            default: default.map(|s| serde_json::Value::String(s)),
            enum_values: None,
        }
    }

    /// Create a required number parameter.
    pub fn required_number(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "number".to_string(),
            required: true,
            default: None,
            enum_values: None,
        }
    }

    /// Create an optional number parameter.
    pub fn optional_number(
        name: impl Into<String>,
        description: impl Into<String>,
        default: Option<f64>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "number".to_string(),
            required: false,
            default: default.map(|n| serde_json::json!(n)),
            enum_values: None,
        }
    }

    /// Create an enum parameter.
    pub fn enum_param(
        name: impl Into<String>,
        description: impl Into<String>,
        values: Vec<String>,
        required: bool,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            param_type: "string".to_string(),
            required,
            default: None,
            enum_values: Some(values),
        }
    }
}

/// Result of tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the execution was successful
    pub success: bool,
    /// Output data (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// Error message (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl ToolResult {
    /// Create a successful result.
    pub fn success(output: serde_json::Value, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            output: Some(output),
            error: None,
            execution_time_ms,
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            output: None,
            error: Some(error.into()),
            execution_time_ms,
        }
    }
}

/// Trait for tool implementations.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name.
    fn name(&self) -> &str;

    /// Get the tool description.
    fn description(&self) -> &str;

    /// Get the parameter definitions.
    fn parameters(&self) -> Vec<ToolParameter>;

    /// Execute the tool with the given parameters.
    async fn execute(&self, params: serde_json::Value) -> AgentResult<ToolResult>;

    /// Generate OpenAI function calling schema.
    fn openai_schema(&self) -> serde_json::Value {
        let properties: serde_json::Map<String, serde_json::Value> = self
            .parameters()
            .iter()
            .map(|p| {
                let mut prop = serde_json::json!({
                    "type": p.param_type,
                    "description": p.description,
                });
                if let Some(ref enums) = p.enum_values {
                    prop["enum"] = serde_json::json!(enums);
                }
                (p.name.clone(), prop)
            })
            .collect();

        let required: Vec<String> = self
            .parameters()
            .iter()
            .filter(|p| p.required)
            .map(|p| p.name.clone())
            .collect();

        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": self.description(),
                "parameters": {
                    "type": "object",
                    "properties": properties,
                    "required": required
                }
            }
        })
    }

    /// Generate Anthropic tool schema.
    fn anthropic_schema(&self) -> serde_json::Value {
        let properties: serde_json::Map<String, serde_json::Value> = self
            .parameters()
            .iter()
            .map(|p| {
                let mut prop = serde_json::json!({
                    "type": p.param_type,
                    "description": p.description,
                });
                if let Some(ref enums) = p.enum_values {
                    prop["enum"] = serde_json::json!(enums);
                }
                (p.name.clone(), prop)
            })
            .collect();

        let required: Vec<String> = self
            .parameters()
            .iter()
            .filter(|p| p.required)
            .map(|p| p.name.clone())
            .collect();

        serde_json::json!({
            "name": self.name(),
            "description": self.description(),
            "input_schema": {
                "type": "object",
                "properties": properties,
                "required": required
            }
        })
    }
}

/// Registry for managing tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create a registry with default tools.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(GetMarketDataTool::new()));
        registry.register(Arc::new(GetAnalysisTool::new()));
        registry.register(Arc::new(PlaceOrderTool::new()));
        registry.register(Arc::new(GetPortfolioTool::new()));
        registry.register(Arc::new(GetSignalsTool::new()));
        registry
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// List all tool names.
    pub fn list(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Execute a tool by name.
    pub async fn execute(
        &self,
        tool_name: &str,
        params: serde_json::Value,
    ) -> AgentResult<ToolResult> {
        let tool = self
            .tools
            .get(tool_name)
            .ok_or_else(|| AgentError::ToolNotFound(tool_name.to_string()))?;

        tool.execute(params).await
    }

    /// Get OpenAI function schemas for all tools.
    pub fn openai_schemas(&self) -> Vec<serde_json::Value> {
        self.tools.values().map(|t| t.openai_schema()).collect()
    }

    /// Get Anthropic tool schemas for all tools.
    pub fn anthropic_schemas(&self) -> Vec<serde_json::Value> {
        self.tools.values().map(|t| t.anthropic_schema()).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// =============================================================================
// Built-in Tools
// =============================================================================

/// Tool for getting market data.
pub struct GetMarketDataTool {
    // In production, this would hold venue clients
}

impl GetMarketDataTool {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for GetMarketDataTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GetMarketDataTool {
    fn name(&self) -> &str {
        "get_market_data"
    }

    fn description(&self) -> &str {
        "Fetch current market data including price, volume, and orderbook for an instrument"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter::required_string("instrument", "Trading instrument (e.g., BTC-PERP)"),
            ToolParameter::enum_param(
                "data_type",
                "Type of data to fetch",
                vec![
                    "ticker".to_string(),
                    "orderbook".to_string(),
                    "candles".to_string(),
                ],
                false,
            ),
        ]
    }

    async fn execute(&self, params: serde_json::Value) -> AgentResult<ToolResult> {
        let start = std::time::Instant::now();

        let instrument = params
            .get("instrument")
            .and_then(|v| v.as_str())
            .unwrap_or("BTC-PERP");
        let data_type = params
            .get("data_type")
            .and_then(|v| v.as_str())
            .unwrap_or("ticker");

        // TODO: In production, fetch from actual venue
        let mock_data = match data_type {
            "ticker" => serde_json::json!({
                "instrument": instrument,
                "price": 50000.0,
                "bid": 49999.0,
                "ask": 50001.0,
                "volume_24h": 1000000000.0,
                "change_24h": 2.5,
                "high_24h": 51000.0,
                "low_24h": 49000.0,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
            "orderbook" => serde_json::json!({
                "instrument": instrument,
                "bids": [[49999.0, 1.5], [49998.0, 2.0], [49997.0, 3.0]],
                "asks": [[50001.0, 1.2], [50002.0, 2.5], [50003.0, 4.0]],
                "timestamp": chrono::Utc::now().to_rfc3339()
            }),
            "candles" => serde_json::json!({
                "instrument": instrument,
                "candles": [
                    {"timestamp": "2024-01-01T00:00:00Z", "open": 50000.0, "high": 50500.0, "low": 49500.0, "close": 50200.0, "volume": 100.0},
                    {"timestamp": "2024-01-01T01:00:00Z", "open": 50200.0, "high": 50700.0, "low": 50000.0, "close": 50400.0, "volume": 120.0},
                ]
            }),
            _ => serde_json::json!({"error": "Unknown data type"}),
        };

        Ok(ToolResult::success(mock_data, start.elapsed().as_millis() as u64))
    }
}

/// Tool for getting technical analysis.
pub struct GetAnalysisTool;

impl GetAnalysisTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetAnalysisTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GetAnalysisTool {
    fn name(&self) -> &str {
        "get_analysis"
    }

    fn description(&self) -> &str {
        "Get technical analysis indicators for an instrument"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter::required_string("instrument", "Trading instrument"),
            ToolParameter {
                name: "indicators".to_string(),
                description: "List of indicators to compute".to_string(),
                param_type: "array".to_string(),
                required: false,
                default: Some(serde_json::json!(["rsi", "macd", "bollinger"])),
                enum_values: None,
            },
        ]
    }

    async fn execute(&self, params: serde_json::Value) -> AgentResult<ToolResult> {
        let start = std::time::Instant::now();

        let instrument = params
            .get("instrument")
            .and_then(|v| v.as_str())
            .unwrap_or("BTC-PERP");

        // TODO: Compute actual indicators
        let mock_analysis = serde_json::json!({
            "instrument": instrument,
            "overall_signal": "neutral",
            "price": 50000.0,
            "indicators": {
                "rsi": 55.0,
                "macd": {
                    "value": 100.0,
                    "signal": 80.0,
                    "histogram": 20.0
                },
                "bollinger": {
                    "upper": 52000.0,
                    "middle": 50000.0,
                    "lower": 48000.0
                },
                "moving_averages": {
                    "sma_20": 49500.0,
                    "sma_50": 48000.0,
                    "ema_12": 49800.0
                }
            },
            "support_resistance": {
                "support": [48000.0, 46000.0, 44000.0],
                "resistance": [52000.0, 55000.0, 60000.0]
            }
        });

        Ok(ToolResult::success(mock_analysis, start.elapsed().as_millis() as u64))
    }
}

/// Tool for placing orders.
pub struct PlaceOrderTool;

impl PlaceOrderTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlaceOrderTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PlaceOrderTool {
    fn name(&self) -> &str {
        "place_order"
    }

    fn description(&self) -> &str {
        "Place a trading order (market or limit)"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![
            ToolParameter::required_string("instrument", "Trading instrument"),
            ToolParameter::enum_param(
                "side",
                "Order side",
                vec!["buy".to_string(), "sell".to_string()],
                true,
            ),
            ToolParameter::enum_param(
                "order_type",
                "Order type",
                vec!["market".to_string(), "limit".to_string()],
                true,
            ),
            ToolParameter::required_number("size", "Order size"),
            ToolParameter::optional_number("price", "Limit price (required for limit orders)", None),
        ]
    }

    async fn execute(&self, params: serde_json::Value) -> AgentResult<ToolResult> {
        let start = std::time::Instant::now();

        let instrument = params.get("instrument").and_then(|v| v.as_str());
        let side = params.get("side").and_then(|v| v.as_str());
        let order_type = params.get("order_type").and_then(|v| v.as_str());
        let size = params.get("size").and_then(|v| v.as_f64());

        // Validate required params
        if instrument.is_none() || side.is_none() || order_type.is_none() || size.is_none() {
            return Ok(ToolResult::failure(
                "Missing required parameters",
                start.elapsed().as_millis() as u64,
            ));
        }

        // TODO: Actually place order via venue
        let mock_result = serde_json::json!({
            "order_id": uuid::Uuid::new_v4().to_string(),
            "instrument": instrument,
            "side": side,
            "order_type": order_type,
            "size": size,
            "status": "filled",
            "fill_price": 50000.0,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        Ok(ToolResult::success(mock_result, start.elapsed().as_millis() as u64))
    }
}

/// Tool for getting portfolio.
pub struct GetPortfolioTool;

impl GetPortfolioTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetPortfolioTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GetPortfolioTool {
    fn name(&self) -> &str {
        "get_portfolio"
    }

    fn description(&self) -> &str {
        "Get current portfolio including positions and P&L"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![]
    }

    async fn execute(&self, _params: serde_json::Value) -> AgentResult<ToolResult> {
        let start = std::time::Instant::now();

        // TODO: Fetch actual portfolio
        let mock_portfolio = serde_json::json!({
            "equity": 100000.0,
            "unrealized_pnl": 500.0,
            "realized_pnl": 1500.0,
            "margin_used": 10000.0,
            "positions": [
                {
                    "instrument": "BTC-PERP",
                    "side": "long",
                    "size": 0.5,
                    "entry_price": 49000.0,
                    "current_price": 50000.0,
                    "unrealized_pnl": 500.0,
                    "leverage": 5.0
                }
            ]
        });

        Ok(ToolResult::success(mock_portfolio, start.elapsed().as_millis() as u64))
    }
}

/// Tool for getting signals.
pub struct GetSignalsTool;

impl GetSignalsTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetSignalsTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GetSignalsTool {
    fn name(&self) -> &str {
        "get_signals"
    }

    fn description(&self) -> &str {
        "Get active trading signals"
    }

    fn parameters(&self) -> Vec<ToolParameter> {
        vec![ToolParameter::optional_string(
            "instrument",
            "Filter by instrument",
            None,
        )]
    }

    async fn execute(&self, _params: serde_json::Value) -> AgentResult<ToolResult> {
        let start = std::time::Instant::now();

        // TODO: Fetch actual signals
        let mock_signals = serde_json::json!({
            "signals": [
                {
                    "instrument": "BTC-PERP",
                    "direction": "long",
                    "strength": 0.75,
                    "signal_type": "momentum",
                    "source": "RSI Strategy",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }
            ]
        });

        Ok(ToolResult::success(mock_signals, start.elapsed().as_millis() as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_market_data_tool() {
        let tool = GetMarketDataTool::new();

        let result = tool
            .execute(serde_json::json!({
                "instrument": "BTC-PERP",
                "data_type": "ticker"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.is_some());
    }

    #[test]
    fn test_tool_registry() {
        let registry = ToolRegistry::with_defaults();

        assert!(registry.get("get_market_data").is_some());
        assert!(registry.get("get_analysis").is_some());
        assert!(registry.get("place_order").is_some());
        assert!(registry.get("unknown_tool").is_none());
    }

    #[test]
    fn test_openai_schema() {
        let tool = GetMarketDataTool::new();
        let schema = tool.openai_schema();

        assert_eq!(schema["type"], "function");
        assert_eq!(schema["function"]["name"], "get_market_data");
    }
}
