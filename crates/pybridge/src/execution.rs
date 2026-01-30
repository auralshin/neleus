use pyo3::prelude::*;
use super::types::*;

#[pyclass(name = "TwapParams")]
#[derive(Debug, Clone)]
pub struct PyTwapParams {
    #[pyo3(get, set)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get, set)]
    pub side: PyOrderSide,
    #[pyo3(get, set)]
    pub total_quantity: f64,
    #[pyo3(get, set)]
    pub duration_secs: u64,
    #[pyo3(get, set)]
    pub num_slices: u32,
    #[pyo3(get, set)]
    pub randomize_timing: bool,
    #[pyo3(get, set)]
    pub limit_price: Option<f64>,
}

#[pymethods]
impl PyTwapParams {
    #[new]
    #[pyo3(signature = (instrument_id, side, total_quantity, duration_secs, num_slices=10, randomize_timing=true, limit_price=None))]
    pub fn new(
        instrument_id: PyInstrumentId,
        side: PyOrderSide,
        total_quantity: f64,
        duration_secs: u64,
        num_slices: u32,
        randomize_timing: bool,
        limit_price: Option<f64>,
    ) -> Self {
        Self {
            instrument_id,
            side,
            total_quantity,
            duration_secs,
            num_slices,
            randomize_timing,
            limit_price,
        }
    }

    pub fn slice_quantity(&self) -> f64 {
        self.total_quantity / self.num_slices as f64
    }

    pub fn slice_interval_secs(&self) -> f64 {
        self.duration_secs as f64 / self.num_slices as f64
    }
}

#[pyclass(name = "VwapParams")]
#[derive(Debug, Clone)]
pub struct PyVwapParams {
    #[pyo3(get, set)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get, set)]
    pub side: PyOrderSide,
    #[pyo3(get, set)]
    pub total_quantity: f64,
    #[pyo3(get, set)]
    pub participation_rate: f64,
    #[pyo3(get, set)]
    pub min_slice: f64,
    #[pyo3(get, set)]
    pub max_slice: f64,
    #[pyo3(get, set)]
    pub limit_price: Option<f64>,
}

#[pymethods]
impl PyVwapParams {
    #[new]
    #[pyo3(signature = (instrument_id, side, total_quantity, participation_rate=0.1, min_slice=0.0, max_slice=1000000.0, limit_price=None))]
    pub fn new(
        instrument_id: PyInstrumentId,
        side: PyOrderSide,
        total_quantity: f64,
        participation_rate: f64,
        min_slice: f64,
        max_slice: f64,
        limit_price: Option<f64>,
    ) -> Self {
        Self {
            instrument_id,
            side,
            total_quantity,
            participation_rate,
            min_slice,
            max_slice,
            limit_price,
        }
    }
}

#[pyclass(name = "IcebergParams")]
#[derive(Debug, Clone)]
pub struct PyIcebergParams {
    #[pyo3(get, set)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get, set)]
    pub side: PyOrderSide,
    #[pyo3(get, set)]
    pub total_quantity: f64,
    #[pyo3(get, set)]
    pub display_quantity: f64,
    #[pyo3(get, set)]
    pub limit_price: f64,
    #[pyo3(get, set)]
    pub variance_pct: f64,
}

#[pymethods]
impl PyIcebergParams {
    #[new]
    #[pyo3(signature = (instrument_id, side, total_quantity, display_quantity, limit_price, variance_pct=0.1))]
    pub fn new(
        instrument_id: PyInstrumentId,
        side: PyOrderSide,
        total_quantity: f64,
        display_quantity: f64,
        limit_price: f64,
        variance_pct: f64,
    ) -> Self {
        Self {
            instrument_id,
            side,
            total_quantity,
            display_quantity,
            limit_price,
            variance_pct,
        }
    }
}

#[pyclass(eq, eq_int, name = "ExecutionState")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyExecutionState {
    Pending,
    Active,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

#[pyclass(eq, eq_int, name = "AdaptiveMode")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyAdaptiveMode {
    Passive,
    Neutral,
    Aggressive,
    Opportunistic,
}

#[pyclass(name = "AdaptiveParams")]
#[derive(Debug, Clone)]
pub struct PyAdaptiveParams {
    #[pyo3(get, set)]
    pub instrument_id: PyInstrumentId,
    #[pyo3(get, set)]
    pub side: PyOrderSide,
    #[pyo3(get, set)]
    pub total_quantity: f64,
    #[pyo3(get, set)]
    pub urgency: f64,
    #[pyo3(get, set)]
    pub risk_aversion: f64,
    #[pyo3(get, set)]
    pub max_duration_secs: u64,
    #[pyo3(get, set)]
    pub limit_price: Option<f64>,
}

#[pymethods]
impl PyAdaptiveParams {
    #[new]
    #[pyo3(signature = (instrument_id, side, total_quantity, urgency=0.5, risk_aversion=0.5, max_duration_secs=0, limit_price=None))]
    pub fn new(
        instrument_id: PyInstrumentId,
        side: PyOrderSide,
        total_quantity: f64,
        urgency: f64,
        risk_aversion: f64,
        max_duration_secs: u64,
        limit_price: Option<f64>,
    ) -> Self {
        Self {
            instrument_id,
            side,
            total_quantity,
            urgency,
            risk_aversion,
            max_duration_secs,
            limit_price,
        }
    }
}

#[pyclass(name = "MarketConditions")]
#[derive(Debug, Clone)]
pub struct PyMarketConditions {
    #[pyo3(get, set)]
    pub bid: f64,
    #[pyo3(get, set)]
    pub ask: f64,
    #[pyo3(get, set)]
    pub bid_size: f64,
    #[pyo3(get, set)]
    pub ask_size: f64,
    #[pyo3(get, set)]
    pub recent_volume: f64,
    #[pyo3(get, set)]
    pub volatility: f64,
}

#[pymethods]
impl PyMarketConditions {
    #[new]
    pub fn new(
        bid: f64,
        ask: f64,
        bid_size: f64,
        ask_size: f64,
        recent_volume: f64,
        volatility: f64,
    ) -> Self {
        Self {
            bid,
            ask,
            bid_size,
            ask_size,
            recent_volume,
            volatility,
        }
    }

    pub fn mid_price(&self) -> f64 {
        (self.bid + self.ask) / 2.0
    }

    pub fn spread_bps(&self) -> f64 {
        let mid = self.mid_price();
        if mid > 0.0 {
            (self.ask - self.bid) / mid * 10000.0
        } else {
            0.0
        }
    }

    pub fn book_imbalance(&self) -> f64 {
        let total = self.bid_size + self.ask_size;
        if total > 0.0 {
            (self.bid_size - self.ask_size) / total
        } else {
            0.0
        }
    }
}
