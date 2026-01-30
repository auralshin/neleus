use crate::node::BacktestResults;
use neleus_core_types::UnixNanos;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardConfig {
    pub num_windows: usize,

    pub in_sample_fraction: f64,

    pub anchored: bool,

    pub min_in_sample_periods: usize,
}

impl Default for WalkForwardConfig {
    fn default() -> Self {
        Self {
            num_windows: 5,
            in_sample_fraction: 0.7,
            anchored: false,
            min_in_sample_periods: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WalkForwardWindow {
    pub window_idx: usize,
    pub train_start: UnixNanos,
    pub train_end: UnixNanos,
    pub test_start: UnixNanos,
    pub test_end: UnixNanos,
}

pub struct WalkForwardSplitter {
    config: WalkForwardConfig,
}

impl WalkForwardSplitter {
    pub fn new(config: WalkForwardConfig) -> Self {
        Self { config }
    }

    pub fn generate_windows(&self, start: UnixNanos, end: UnixNanos) -> Vec<WalkForwardWindow> {
        let total_nanos = end.as_nanos() - start.as_nanos();
        let mut windows = Vec::new();

        if self.config.anchored {
            let test_window_size = total_nanos / (self.config.num_windows as u128 + 1);

            for i in 0..self.config.num_windows {
                let test_start_nanos = start.as_nanos()
                    + (total_nanos * (i + 1) as u128) / (self.config.num_windows + 1) as u128;
                let test_end_nanos = test_start_nanos + test_window_size;

                windows.push(WalkForwardWindow {
                    window_idx: i,
                    train_start: start,
                    train_end: UnixNanos::from_nanos(test_start_nanos as u64),
                    test_start: UnixNanos::from_nanos(test_start_nanos as u64),
                    test_end: UnixNanos::from_nanos(test_end_nanos.min(end.as_nanos()) as u64),
                });
            }
        } else {
            let window_size = total_nanos / self.config.num_windows as u128;
            let train_size = (window_size as f64 * self.config.in_sample_fraction) as u128;

            for i in 0..self.config.num_windows {
                let window_start = start.as_nanos() + (window_size * i as u128);
                let train_end_nanos = window_start + train_size;
                let window_end = window_start + window_size;

                windows.push(WalkForwardWindow {
                    window_idx: i,
                    train_start: UnixNanos::from_nanos(window_start as u64),
                    train_end: UnixNanos::from_nanos(train_end_nanos as u64),
                    test_start: UnixNanos::from_nanos(train_end_nanos as u64),
                    test_end: UnixNanos::from_nanos(window_end.min(end.as_nanos()) as u64),
                });
            }
        }

        windows
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardResult {
    pub window_idx: usize,
    pub train_start: UnixNanos,
    pub train_end: UnixNanos,
    pub test_start: UnixNanos,
    pub test_end: UnixNanos,

    pub best_params: HashMap<String, f64>,

    pub train_results: BacktestResults,

    pub test_results: BacktestResults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkForwardAnalysis {
    pub windows: Vec<WalkForwardResult>,

    pub combined_oos_pnl: f64,
    pub combined_oos_return_pct: f64,
    pub combined_oos_sharpe: f64,

    pub robustness_ratio: f64,
}

impl WalkForwardAnalysis {
    pub fn from_results(windows: Vec<WalkForwardResult>) -> Self {
        let combined_oos_pnl: f64 = windows.iter().map(|w| w.test_results.total_pnl).sum();

        let initial_balance: f64 = windows
            .first()
            .map(|w| w.test_results.initial_balance)
            .unwrap_or(100_000.0);

        let combined_oos_return_pct = (combined_oos_pnl / initial_balance) * 100.0;

        let oos_sharpes: Vec<_> = windows
            .iter()
            .map(|w| w.test_results.sharpe_ratio)
            .filter(|s| s.is_finite())
            .collect();
        let combined_oos_sharpe = if !oos_sharpes.is_empty() {
            oos_sharpes.iter().sum::<f64>() / oos_sharpes.len() as f64
        } else {
            0.0
        };

        let is_sharpes: Vec<_> = windows
            .iter()
            .map(|w| w.train_results.sharpe_ratio)
            .filter(|s| s.is_finite() && *s > 0.0)
            .collect();
        let avg_is_sharpe = if !is_sharpes.is_empty() {
            is_sharpes.iter().sum::<f64>() / is_sharpes.len() as f64
        } else {
            1.0
        };

        let robustness_ratio = if avg_is_sharpe > 0.0 {
            combined_oos_sharpe / avg_is_sharpe
        } else {
            0.0
        };

        Self {
            windows,
            combined_oos_pnl,
            combined_oos_return_pct,
            combined_oos_sharpe,
            robustness_ratio,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

impl ParameterDef {
    pub fn new(name: impl Into<String>, min: f64, max: f64, step: f64) -> Self {
        Self {
            name: name.into(),
            min,
            max,
            step,
        }
    }

    pub fn values(&self) -> Vec<f64> {
        let mut vals = Vec::new();
        let mut v = self.min;
        while v <= self.max + 1e-10 {
            vals.push(v);
            v += self.step;
        }
        vals
    }

    pub fn count(&self) -> usize {
        self.values().len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSweepConfig {
    pub parameters: Vec<ParameterDef>,

    pub target_metric: OptimizationMetric,

    pub maximize: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationMetric {
    TotalPnl,
    ReturnPct,
    SharpeRatio,
    SortinoRatio,
    CalmarRatio,
    ProfitFactor,
    WinRate,
    MaxDrawdown,
}

impl OptimizationMetric {
    pub fn extract(&self, results: &BacktestResults) -> f64 {
        match self {
            Self::TotalPnl => results.total_pnl,
            Self::ReturnPct => results.return_pct,
            Self::SharpeRatio => results.sharpe_ratio,
            Self::SortinoRatio => results.sortino_ratio,
            Self::CalmarRatio => results.calmar_ratio,
            Self::ProfitFactor => results.profit_factor,
            Self::WinRate => results.win_rate,
            Self::MaxDrawdown => results.max_drawdown_pct,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepResult {
    pub params: HashMap<String, f64>,
    pub metric_value: f64,
    pub results: BacktestResults,
}

pub struct ParameterSweep {
    config: ParameterSweepConfig,
}

impl ParameterSweep {
    pub fn new(config: ParameterSweepConfig) -> Self {
        Self { config }
    }

    pub fn generate_combinations(&self) -> Vec<HashMap<String, f64>> {
        let mut combos = vec![HashMap::new()];

        for param in &self.config.parameters {
            let mut new_combos = Vec::new();
            for combo in &combos {
                for value in param.values() {
                    let mut new_combo = combo.clone();
                    new_combo.insert(param.name.clone(), value);
                    new_combos.push(new_combo);
                }
            }
            combos = new_combos;
        }

        combos
    }

    pub fn total_combinations(&self) -> usize {
        self.config.parameters.iter().map(|p| p.count()).product()
    }

    pub fn find_best<'a>(&self, results: &'a [SweepResult]) -> Option<&'a SweepResult> {
        if results.is_empty() {
            return None;
        }

        if self.config.maximize {
            results.iter().max_by(|a, b| {
                a.metric_value
                    .partial_cmp(&b.metric_value)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        } else {
            results.iter().min_by(|a, b| {
                a.metric_value
                    .partial_cmp(&b.metric_value)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        }
    }

    pub fn parameter_sensitivity(
        &self,
        results: &[SweepResult],
    ) -> HashMap<String, ParameterSensitivity> {
        let mut sensitivities = HashMap::new();

        for param in &self.config.parameters {
            let mut value_metrics: HashMap<String, Vec<f64>> = HashMap::new();

            for result in results {
                if let Some(val) = result.params.get(&param.name) {
                    let key = format!("{:.6}", val);
                    value_metrics
                        .entry(key)
                        .or_default()
                        .push(result.metric_value);
                }
            }

            let mut averages: Vec<(f64, f64)> = value_metrics
                .iter()
                .map(|(k, v)| {
                    let param_val: f64 = k.parse().unwrap_or(0.0);
                    let avg_metric = v.iter().sum::<f64>() / v.len() as f64;
                    (param_val, avg_metric)
                })
                .collect();
            averages.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            let best = if self.config.maximize {
                averages
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            } else {
                averages
                    .iter()
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            };

            let metric_range = averages.iter().map(|(_, m)| *m).fold(f64::NAN, f64::max)
                - averages.iter().map(|(_, m)| *m).fold(f64::NAN, f64::min);

            sensitivities.insert(
                param.name.clone(),
                ParameterSensitivity {
                    param_name: param.name.clone(),
                    best_value: best.map(|(v, _)| *v).unwrap_or(param.min),
                    best_metric: best.map(|(_, m)| *m).unwrap_or(0.0),
                    metric_range,
                    value_metrics: averages,
                },
            );
        }

        sensitivities
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSensitivity {
    pub param_name: String,
    pub best_value: f64,
    pub best_metric: f64,
    pub metric_range: f64,

    pub value_metrics: Vec<(f64, f64)>,
}
