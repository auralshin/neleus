use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Venue {
    Hyperliquid,
    Lighter,

    Simulated,
}

impl fmt::Display for Venue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Venue::Hyperliquid => write!(f, "HYPERLIQUID"),
            Venue::Lighter => write!(f, "LIGHTER"),
            Venue::Simulated => write!(f, "SIMULATED"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstrumentType {
    Perp,
    Spot,
}

impl fmt::Display for InstrumentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstrumentType::Perp => write!(f, "PERP"),
            InstrumentType::Spot => write!(f, "SPOT"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstrumentId {
    pub venue: Venue,
    pub symbol: String,
    pub kind: InstrumentType,
}

impl InstrumentId {
    pub fn new(venue: Venue, symbol: impl Into<String>, kind: InstrumentType) -> Self {
        Self {
            venue,
            symbol: symbol.into(),
            kind,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        let venue = match parts[0].to_uppercase().as_str() {
            "HYPERLIQUID" => Venue::Hyperliquid,
            "LIGHTER" => Venue::Lighter,
            "SIMULATED" => Venue::Simulated,
            _ => return None,
        };
        let sym_parts: Vec<&str> = parts[1].split('.').collect();
        if sym_parts.len() != 2 {
            return None;
        }
        let kind = match sym_parts[1].to_uppercase().as_str() {
            "PERP" => InstrumentType::Perp,
            "SPOT" => InstrumentType::Spot,
            _ => return None,
        };
        Some(Self {
            venue,
            symbol: sym_parts[0].to_string(),
            kind,
        })
    }
}

impl fmt::Display for InstrumentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}.{}", self.venue, self.symbol, self.kind)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct UnixNanos(pub u64);

impl UnixNanos {
    pub const ZERO: UnixNanos = UnixNanos(0);

    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        Self(duration.as_nanos() as u64)
    }

    pub fn from_millis(ms: u64) -> Self {
        Self(ms * 1_000_000)
    }

    pub fn from_micros(us: u64) -> Self {
        Self(us * 1_000)
    }

    pub fn from_nanos(ns: u64) -> Self {
        Self(ns)
    }

    pub fn from_secs(secs: u64) -> Self {
        Self(secs * 1_000_000_000)
    }

    pub fn as_millis(&self) -> u64 {
        self.0 / 1_000_000
    }

    pub fn as_micros(&self) -> u64 {
        self.0 / 1_000
    }

    pub fn as_secs(&self) -> u64 {
        self.0 / 1_000_000_000
    }

    pub fn as_nanos(&self) -> u128 {
        self.0 as u128
    }
}

impl Add for UnixNanos {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for UnixNanos {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventTimestamps {
    pub ts_event: UnixNanos,

    pub ts_recv: UnixNanos,

    pub ts_proc: UnixNanos,
}

impl EventTimestamps {
    pub fn new(ts_event: UnixNanos, ts_recv: UnixNanos, ts_proc: UnixNanos) -> Self {
        Self {
            ts_event,
            ts_recv,
            ts_proc,
        }
    }

    pub fn simulated(ts: UnixNanos) -> Self {
        Self {
            ts_event: ts,
            ts_recv: ts,
            ts_proc: ts,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct FixedPoint {
    pub value: i64,

    pub scale: u8,
}

impl FixedPoint {
    pub const ZERO: FixedPoint = FixedPoint { value: 0, scale: 8 };

    pub fn new(value: i64, scale: u8) -> Self {
        Self { value, scale }
    }

    pub fn from_f64(f: f64, scale: u8) -> Self {
        let multiplier = 10_i64.pow(scale as u32);
        Self {
            value: (f * multiplier as f64).round() as i64,
            scale,
        }
    }

    pub fn from_str(s: &str, scale: u8) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        let multiplier = 10_i64.pow(scale as u32);

        match parts.len() {
            1 => {
                let int_part: i64 = parts[0].parse().ok()?;
                Some(Self {
                    value: int_part * multiplier,
                    scale,
                })
            }
            2 => {
                let negative = parts[0].starts_with('-');
                let int_part: i64 = parts[0].parse().ok()?;
                let frac_str = parts[1];
                let frac_scale = frac_str.len() as u32;

                let frac_value: i64 = if frac_scale <= scale as u32 {
                    let padding = 10_i64.pow(scale as u32 - frac_scale);
                    frac_str.parse::<i64>().ok()? * padding
                } else {
                    let divisor = 10_i64.pow(frac_scale - scale as u32);
                    frac_str.parse::<i64>().ok()? / divisor
                };

                let value = if negative {
                    int_part * multiplier - frac_value
                } else {
                    int_part * multiplier + frac_value
                };

                Some(Self { value, scale })
            }
            _ => None,
        }
    }

    pub fn to_f64(&self) -> f64 {
        self.value as f64 / 10_f64.powi(self.scale as i32)
    }

    pub fn is_zero(&self) -> bool {
        self.value == 0
    }

    pub fn is_positive(&self) -> bool {
        self.value > 0
    }

    pub fn is_negative(&self) -> bool {
        self.value < 0
    }

    pub fn abs(&self) -> Self {
        Self {
            value: self.value.abs(),
            scale: self.scale,
        }
    }

    pub fn rescale(&self, new_scale: u8) -> Self {
        if new_scale == self.scale {
            return *self;
        }
        let value = if new_scale > self.scale {
            self.value * 10_i64.pow((new_scale - self.scale) as u32)
        } else {
            self.value / 10_i64.pow((self.scale - new_scale) as u32)
        };
        Self {
            value,
            scale: new_scale,
        }
    }
}

impl fmt::Display for FixedPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let divisor = 10_f64.powi(self.scale as i32);
        write!(
            f,
            "{:.prec$}",
            self.value as f64 / divisor,
            prec = self.scale as usize
        )
    }
}

impl Add for FixedPoint {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        assert_eq!(
            self.scale, rhs.scale,
            "Scale mismatch in FixedPoint addition"
        );
        Self {
            value: self.value + rhs.value,
            scale: self.scale,
        }
    }
}

impl Sub for FixedPoint {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        assert_eq!(
            self.scale, rhs.scale,
            "Scale mismatch in FixedPoint subtraction"
        );
        Self {
            value: self.value - rhs.value,
            scale: self.scale,
        }
    }
}

impl Mul for FixedPoint {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        let new_value = self.value * rhs.value / 10_i64.pow(self.scale as u32);
        Self {
            value: new_value,
            scale: self.scale,
        }
    }
}

impl Div for FixedPoint {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        assert!(!rhs.is_zero(), "Division by zero");
        let new_value = self.value * 10_i64.pow(self.scale as u32) / rhs.value;
        Self {
            value: new_value,
            scale: self.scale,
        }
    }
}

impl Neg for FixedPoint {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self {
            value: -self.value,
            scale: self.scale,
        }
    }
}

impl PartialOrd for FixedPoint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FixedPoint {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.scale == other.scale {
            self.value.cmp(&other.value)
        } else {
            let self_rescaled = self.rescale(8);
            let other_rescaled = other.rescale(8);
            self_rescaled.value.cmp(&other_rescaled.value)
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct Price(pub FixedPoint);

impl Price {
    pub const ZERO: Price = Price(FixedPoint::ZERO);

    pub fn new(value: i64, scale: u8) -> Self {
        Self(FixedPoint::new(value, scale))
    }

    pub fn from_f64(f: f64, scale: u8) -> Self {
        Self(FixedPoint::from_f64(f, scale))
    }

    pub fn from_str(s: &str, scale: u8) -> Option<Self> {
        FixedPoint::from_str(s, scale).map(Self)
    }

    pub fn to_f64(&self) -> f64 {
        self.0.to_f64()
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn inner(&self) -> FixedPoint {
        self.0
    }
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct Quantity(pub FixedPoint);

impl Quantity {
    pub const ZERO: Quantity = Quantity(FixedPoint::ZERO);

    pub fn new(value: i64, scale: u8) -> Self {
        Self(FixedPoint::new(value, scale))
    }

    pub fn from_f64(f: f64, scale: u8) -> Self {
        Self(FixedPoint::from_f64(f, scale))
    }

    pub fn from_str(s: &str, scale: u8) -> Option<Self> {
        FixedPoint::from_str(s, scale).map(Self)
    }

    pub fn to_f64(&self) -> f64 {
        self.0.to_f64()
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn inner(&self) -> FixedPoint {
        self.0
    }

    pub fn abs(&self) -> Self {
        Self(self.0.abs())
    }
}

impl Add for Quantity {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Quantity {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Neg for Quantity {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Money {
    pub amount: FixedPoint,
    pub currency: Currency,
}

impl Money {
    pub fn new(amount: FixedPoint, currency: Currency) -> Self {
        Self { amount, currency }
    }

    pub fn usd(amount: FixedPoint) -> Self {
        Self {
            amount,
            currency: Currency::USD,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.amount.is_zero()
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Currency {
    #[default]
    USD,
    USDC,
    USDT,
    ETH,
    BTC,
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Currency::USD => write!(f, "USD"),
            Currency::USDC => write!(f, "USDC"),
            Currency::USDT => write!(f, "USDT"),
            Currency::ETH => write!(f, "ETH"),
            Currency::BTC => write!(f, "BTC"),
        }
    }
}

macro_rules! define_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn generate() -> Self {
                use std::time::{SystemTime, UNIX_EPOCH};
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                let random: u64 = rand_simple();
                Self(format!("{:012x}{:016x}", timestamp as u64, random))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
    };
}

fn rand_simple() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let mut x = seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

define_id!(OrderId, "Internal order identifier (UUID/ULID format)");
define_id!(VenueOrderId, "Venue-assigned order identifier");
define_id!(ClientOrderId, "Client-assigned order identifier");
define_id!(TradeId, "Trade/execution identifier");
define_id!(PositionId, "Position identifier");
define_id!(AccountId, "Account identifier");
define_id!(StrategyId, "Strategy identifier");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl OrderSide {
    pub fn opposite(&self) -> Self {
        match self {
            OrderSide::Buy => OrderSide::Sell,
            OrderSide::Sell => OrderSide::Buy,
        }
    }

    pub fn sign(&self) -> i64 {
        match self {
            OrderSide::Buy => 1,
            OrderSide::Sell => -1,
        }
    }
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "BUY"),
            OrderSide::Sell => write!(f, "SELL"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TimeInForce {
    #[default]
    GTC,
    IOC,
    FOK,
    GTD,
    DAY,
}

impl fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeInForce::GTC => write!(f, "GTC"),
            TimeInForce::IOC => write!(f, "IOC"),
            TimeInForce::FOK => write!(f, "FOK"),
            TimeInForce::GTD => write!(f, "GTD"),
            TimeInForce::DAY => write!(f, "DAY"),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct SequenceNumber(pub u64);

impl SequenceNumber {
    pub fn new(n: u64) -> Self {
        Self(n)
    }

    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_point_from_str() {
        let fp = FixedPoint::from_str("123.456", 8).unwrap();
        assert_eq!(fp.value, 12345600000);
        assert_eq!(fp.scale, 8);

        let fp2 = FixedPoint::from_str("100", 8).unwrap();
        assert_eq!(fp2.value, 10000000000);
    }

    #[test]
    fn test_fixed_point_arithmetic() {
        let a = FixedPoint::new(100_00000000, 8);
        let b = FixedPoint::new(50_00000000, 8);

        let sum = a + b;
        assert_eq!(sum.value, 150_00000000);

        let diff = a - b;
        assert_eq!(diff.value, 50_00000000);
    }

    #[test]
    fn test_instrument_id_parse() {
        let id = InstrumentId::parse("HYPERLIQUID:BTC.PERP").unwrap();
        assert_eq!(id.venue, Venue::Hyperliquid);
        assert_eq!(id.symbol, "BTC");
        assert_eq!(id.kind, InstrumentType::Perp);
    }

    #[test]
    fn test_order_id_generate() {
        let id1 = OrderId::generate();
        let id2 = OrderId::generate();
        assert_ne!(id1, id2);
    }
}
