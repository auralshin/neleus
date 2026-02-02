//! Agent configuration types.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{AgentError, AgentResult};

/// Agent personality configuration (from personality.yaml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityConfig {
    /// Agent name
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Trading style: aggressive, balanced, conservative, scalping, swing, position
    #[serde(default = "default_trading_style")]
    pub trading_style: String,
    /// Risk tolerance: low, medium, high
    #[serde(default = "default_risk_tolerance")]
    pub risk_tolerance: String,
    /// Decision speed: fast, deliberate, adaptive
    #[serde(default = "default_decision_speed")]
    pub decision_speed: String,
    /// Behavioral traits
    #[serde(default)]
    pub traits: Vec<String>,
    /// Use technical analysis
    #[serde(default = "default_true")]
    pub use_technical_analysis: bool,
    /// Use fundamental analysis
    #[serde(default)]
    pub use_fundamental_analysis: bool,
    /// Use sentiment analysis
    #[serde(default)]
    pub use_sentiment_analysis: bool,
    /// Prefer momentum strategies
    #[serde(default)]
    pub prefer_momentum: bool,
    /// Prefer mean reversion strategies
    #[serde(default)]
    pub prefer_mean_reversion: bool,
    /// Verbose reasoning in decisions
    #[serde(default = "default_true")]
    pub verbose_reasoning: bool,
    /// Explain decisions in logs
    #[serde(default = "default_true")]
    pub explain_decisions: bool,
    /// Custom system prompt override
    #[serde(default)]
    pub system_prompt: Option<String>,
}

fn default_trading_style() -> String {
    "balanced".to_string()
}

fn default_risk_tolerance() -> String {
    "medium".to_string()
}

fn default_decision_speed() -> String {
    "adaptive".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for PersonalityConfig {
    fn default() -> Self {
        Self {
            name: "Trading Agent".to_string(),
            description: "An AI trading agent".to_string(),
            trading_style: default_trading_style(),
            risk_tolerance: default_risk_tolerance(),
            decision_speed: default_decision_speed(),
            traits: vec!["analytical".to_string(), "risk-aware".to_string()],
            use_technical_analysis: true,
            use_fundamental_analysis: false,
            use_sentiment_analysis: false,
            prefer_momentum: false,
            prefer_mean_reversion: false,
            verbose_reasoning: true,
            explain_decisions: true,
            system_prompt: None,
        }
    }
}

impl PersonalityConfig {
    /// Load from a YAML file.
    pub fn from_file(path: impl AsRef<Path>) -> AgentResult<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_yaml::from_str(&content)
            .map_err(|e| AgentError::Configuration(format!("Invalid personality.yaml: {}", e)))
    }

    /// Generate a system prompt based on personality.
    pub fn generate_system_prompt(&self) -> String {
        if let Some(ref custom) = self.system_prompt {
            return custom.clone();
        }

        let traits_str = if self.traits.is_empty() {
            "analytical and data-driven".to_string()
        } else {
            self.traits.join(", ")
        };

        let analysis_str = {
            let mut methods = Vec::new();
            if self.use_technical_analysis {
                methods.push("technical analysis");
            }
            if self.use_fundamental_analysis {
                methods.push("fundamental analysis");
            }
            if self.use_sentiment_analysis {
                methods.push("sentiment analysis");
            }
            if methods.is_empty() {
                "data analysis".to_string()
            } else {
                methods.join(", ")
            }
        };

        format!(
            r#"You are {}, {}.

Your trading style is {} with {} risk tolerance.
You make decisions in a {} manner.

Your key traits are: {}.

You primarily use {} to make trading decisions.

{}{}

Always:
- Explain your reasoning clearly
- Consider risk before reward
- Be specific about entry/exit points
- State your confidence level
"#,
            self.name,
            self.description,
            self.trading_style,
            self.risk_tolerance,
            self.decision_speed,
            traits_str,
            analysis_str,
            if self.prefer_momentum {
                "You prefer momentum-based strategies. "
            } else {
                ""
            },
            if self.prefer_mean_reversion {
                "You prefer mean reversion strategies. "
            } else {
                ""
            },
        )
    }
}

/// Agent info configuration (from info.yaml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoConfig {
    /// Version
    #[serde(default = "default_version")]
    pub version: String,
    /// LLM provider: openai, anthropic, ollama
    #[serde(default = "default_llm_provider")]
    pub llm_provider: String,
    /// LLM model name
    #[serde(default = "default_llm_model")]
    pub llm_model: String,
    /// Temperature for LLM
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    /// Max tokens for LLM response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// Available tools
    #[serde(default)]
    pub tools: Vec<String>,
    /// Supported instruments
    #[serde(default)]
    pub instruments: Vec<String>,
    /// Supported venues
    #[serde(default)]
    pub venues: Vec<String>,
    /// Data feeds
    #[serde(default)]
    pub data_feeds: Vec<String>,
    /// Max position size as fraction of portfolio
    #[serde(default = "default_max_position")]
    pub max_position_size: f64,
    /// Max daily loss as fraction of portfolio
    #[serde(default = "default_max_daily_loss")]
    pub max_daily_loss: f64,
    /// Max leverage
    #[serde(default = "default_max_leverage")]
    pub max_leverage: f64,
    /// Decision interval in seconds
    #[serde(default = "default_decision_interval")]
    pub decision_interval_seconds: u64,
    /// Max decisions per hour
    #[serde(default = "default_max_decisions")]
    pub max_decisions_per_hour: u32,
    /// Memory backend: local, redis, postgres
    #[serde(default = "default_memory_backend")]
    pub memory_backend: String,
    /// Vector store: chromadb, pinecone, null
    #[serde(default)]
    pub vector_store: Option<String>,
    /// Knowledge sources
    #[serde(default)]
    pub knowledge_sources: Vec<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_llm_provider() -> String {
    "openai".to_string()
}

fn default_llm_model() -> String {
    "gpt-4o".to_string()
}

fn default_temperature() -> f64 {
    0.7
}

fn default_max_tokens() -> usize {
    4096
}

fn default_max_position() -> f64 {
    0.1
}

fn default_max_daily_loss() -> f64 {
    0.05
}

fn default_max_leverage() -> f64 {
    5.0
}

fn default_decision_interval() -> u64 {
    60
}

fn default_max_decisions() -> u32 {
    60
}

fn default_memory_backend() -> String {
    "local".to_string()
}

impl Default for InfoConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            llm_provider: default_llm_provider(),
            llm_model: default_llm_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            tools: vec![
                "get_market_data".to_string(),
                "get_analysis".to_string(),
                "place_order".to_string(),
                "get_portfolio".to_string(),
            ],
            instruments: vec!["BTC-PERP".to_string()],
            venues: vec!["hyperliquid".to_string()],
            data_feeds: vec![],
            max_position_size: default_max_position(),
            max_daily_loss: default_max_daily_loss(),
            max_leverage: default_max_leverage(),
            decision_interval_seconds: default_decision_interval(),
            max_decisions_per_hour: default_max_decisions(),
            memory_backend: default_memory_backend(),
            vector_store: None,
            knowledge_sources: vec![],
        }
    }
}

impl InfoConfig {
    /// Load from a YAML file.
    pub fn from_file(path: impl AsRef<Path>) -> AgentResult<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_yaml::from_str(&content)
            .map_err(|e| AgentError::Configuration(format!("Invalid info.yaml: {}", e)))
    }
}

/// Combined agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Personality configuration
    pub personality: PersonalityConfig,
    /// Info/capabilities configuration
    pub info: InfoConfig,
    /// Project directory path
    #[serde(skip)]
    pub project_path: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            personality: PersonalityConfig::default(),
            info: InfoConfig::default(),
            project_path: None,
        }
    }
}

impl AgentConfig {
    /// Load configuration from a project directory.
    pub fn from_project(project_path: impl AsRef<Path>) -> AgentResult<Self> {
        let path = project_path.as_ref();

        let personality_path = path.join("personality.yaml");
        let info_path = path.join("info.yaml");

        let personality = if personality_path.exists() {
            PersonalityConfig::from_file(&personality_path)?
        } else {
            return Err(AgentError::Configuration(format!(
                "personality.yaml not found in {}",
                path.display()
            )));
        };

        let info = if info_path.exists() {
            InfoConfig::from_file(&info_path)?
        } else {
            return Err(AgentError::Configuration(format!(
                "info.yaml not found in {}",
                path.display()
            )));
        };

        Ok(Self {
            personality,
            info,
            project_path: Some(path.to_string_lossy().to_string()),
        })
    }

    /// Get the agent's name.
    pub fn name(&self) -> &str {
        &self.personality.name
    }

    /// Generate the system prompt for the LLM.
    pub fn system_prompt(&self) -> String {
        self.personality.generate_system_prompt()
    }
}

// Add serde_yaml as a dev dependency substitute for now
mod serde_yaml {
    use serde::de::DeserializeOwned;

    pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T, String> {
        // Simple YAML parser - in production use real serde_yaml
        // For now, try parsing as JSON (YAML is a superset)
        serde_json::from_str(s).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_personality() {
        let config = PersonalityConfig::default();
        assert_eq!(config.trading_style, "balanced");
        assert_eq!(config.risk_tolerance, "medium");
    }

    #[test]
    fn test_generate_system_prompt() {
        let config = PersonalityConfig::default();
        let prompt = config.generate_system_prompt();
        
        assert!(prompt.contains("Trading Agent"));
        assert!(prompt.contains("balanced"));
    }

    #[test]
    fn test_default_info() {
        let config = InfoConfig::default();
        assert_eq!(config.llm_provider, "openai");
        assert!(config.tools.contains(&"get_market_data".to_string()));
    }
}
