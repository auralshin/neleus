use neleus_core_types::UnixNanos;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestConfig {
    pub start_time: UnixNanos,

    pub end_time: UnixNanos,

    pub sim_mode: SimulationMode,

    pub fill_model: FillModelConfig,

    pub latency_model: LatencyModelConfig,

    pub initial_balance: f64,

    pub commission_rate: f64,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            start_time: UnixNanos::ZERO,
            end_time: UnixNanos::from_millis(u64::MAX / 1_000_000),
            sim_mode: SimulationMode::TradeBased,
            fill_model: FillModelConfig::default(),
            latency_model: LatencyModelConfig::default(),
            initial_balance: 100_000.0,
            commission_rate: 0.0004,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationMode {
    BarBased,

    TradeBased,

    OrderBookBased,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillModelType {
    Immediate,

    NextTick,

    Probabilistic,

    OrderBook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillModelConfig {
    pub model_type: FillModelType,

    pub slippage_bps: u32,

    pub partial_fills: bool,

    pub max_fill_rate: f64,

    pub fill_probability: f64,
}

impl Default for FillModelConfig {
    fn default() -> Self {
        Self {
            model_type: FillModelType::Immediate,
            slippage_bps: 10,
            partial_fills: false,
            max_fill_rate: 1.0,
            fill_probability: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LatencyModelType {
    Zero,

    Fixed,

    Uniform,

    LogNormal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyModelConfig {
    pub model_type: LatencyModelType,

    pub order_latency_ns: u64,

    pub data_latency_ns: u64,

    pub enable_jitter: bool,

    pub jitter_ns: u64,

    pub min_latency_ns: u64,

    pub max_latency_ns: u64,
}

impl Default for LatencyModelConfig {
    fn default() -> Self {
        Self {
            model_type: LatencyModelType::Zero,
            order_latency_ns: 1_000_000,
            data_latency_ns: 500_000,
            enable_jitter: false,
            jitter_ns: 100_000,
            min_latency_ns: 500_000,
            max_latency_ns: 2_000_000,
        }
    }
}

pub struct LatencySimulator {
    config: LatencyModelConfig,
    rng: rand::rngs::ThreadRng,
}

impl LatencySimulator {
    pub fn new(config: LatencyModelConfig) -> Self {
        Self {
            config,
            rng: rand::thread_rng(),
        }
    }

    pub fn order_latency(&mut self) -> u64 {
        self.simulate_latency(self.config.order_latency_ns)
    }

    pub fn data_latency(&mut self) -> u64 {
        self.simulate_latency(self.config.data_latency_ns)
    }

    fn simulate_latency(&mut self, base: u64) -> u64 {
        match self.config.model_type {
            LatencyModelType::Zero => 0,
            LatencyModelType::Fixed => {
                if self.config.enable_jitter {
                    let jitter = self.rng.gen_range(0..self.config.jitter_ns);
                    base + jitter
                } else {
                    base
                }
            }
            LatencyModelType::Uniform => self
                .rng
                .gen_range(self.config.min_latency_ns..=self.config.max_latency_ns),
            LatencyModelType::LogNormal => {
                let u: f64 = self.rng.gen();
                let factor = (-2.0 * u.ln()).sqrt()
                    * (2.0 * std::f64::consts::PI * self.rng.gen::<f64>()).cos();
                let latency = base as f64 * (1.0 + 0.3 * factor).max(0.1);
                latency as u64
            }
        }
    }
}
