use neleus_core_types::{
    AccountId, Currency, FixedPoint, InstrumentId, Money, OrderId, OrderSide, PositionId, Price,
    Quantity, SequenceNumber, StrategyId, TimeInForce, TradeId, UnixNanos, Venue, VenueOrderId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    Market,
    Limit,
    StopMarket,
    StopLimit,
    TakeProfit,
    TrailingStop,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::Market => write!(f, "MARKET"),
            OrderType::Limit => write!(f, "LIMIT"),
            OrderType::StopMarket => write!(f, "STOP_MARKET"),
            OrderType::StopLimit => write!(f, "STOP_LIMIT"),
            OrderType::TakeProfit => write!(f, "TAKE_PROFIT"),
            OrderType::TrailingStop => write!(f, "TRAILING_STOP"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderState {
    New,

    Submitted,

    Accepted,

    PartiallyFilled,

    Filled,

    PendingCancel,

    PendingModify,

    Canceled,

    Rejected,

    Expired,
}

impl OrderState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            OrderState::Filled | OrderState::Canceled | OrderState::Rejected | OrderState::Expired
        )
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self,
            OrderState::Submitted | OrderState::Accepted | OrderState::PartiallyFilled
        )
    }

    pub fn can_transition_to(&self, next: OrderState) -> bool {
        use OrderState::*;
        match (self, next) {
            (New, Submitted) => true,
            (New, Rejected) => true,

            (Submitted, Accepted) => true,
            (Submitted, Rejected) => true,
            (Submitted, Filled) => true,
            (Submitted, PartiallyFilled) => true,
            (Submitted, PendingCancel) => true,

            (Accepted, PartiallyFilled) => true,
            (Accepted, Filled) => true,
            (Accepted, PendingCancel) => true,
            (Accepted, PendingModify) => true,
            (Accepted, Canceled) => true,
            (Accepted, Expired) => true,

            (PartiallyFilled, PartiallyFilled) => true,
            (PartiallyFilled, Filled) => true,
            (PartiallyFilled, PendingCancel) => true,
            (PartiallyFilled, PendingModify) => true,
            (PartiallyFilled, Canceled) => true,

            (PendingCancel, Canceled) => true,
            (PendingCancel, Filled) => true,
            (PendingCancel, PartiallyFilled) => true,
            (PendingCancel, Rejected) => true,

            (PendingModify, Accepted) => true,
            (PendingModify, Rejected) => true,
            (PendingModify, Filled) => true,
            (PendingModify, PartiallyFilled) => true,
            (PendingModify, PendingCancel) => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for OrderState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderState::New => write!(f, "NEW"),
            OrderState::Submitted => write!(f, "SUBMITTED"),
            OrderState::Accepted => write!(f, "ACCEPTED"),
            OrderState::PartiallyFilled => write!(f, "PARTIALLY_FILLED"),
            OrderState::Filled => write!(f, "FILLED"),
            OrderState::PendingCancel => write!(f, "PENDING_CANCEL"),
            OrderState::PendingModify => write!(f, "PENDING_MODIFY"),
            OrderState::Canceled => write!(f, "CANCELED"),
            OrderState::Rejected => write!(f, "REJECTED"),
            OrderState::Expired => write!(f, "EXPIRED"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub venue_order_id: Option<VenueOrderId>,
    pub instrument_id: InstrumentId,
    pub strategy_id: StrategyId,
    pub account_id: AccountId,

    pub side: OrderSide,
    pub order_type: OrderType,
    pub time_in_force: TimeInForce,
    pub price: Option<Price>,
    pub trigger_price: Option<Price>,
    pub quantity: Quantity,
    pub reduce_only: bool,
    pub post_only: bool,

    pub state: OrderState,
    pub filled_quantity: Quantity,
    pub average_fill_price: Option<Price>,

    pub ts_created: UnixNanos,
    pub ts_submitted: Option<UnixNanos>,
    pub ts_accepted: Option<UnixNanos>,
    pub ts_last_fill: Option<UnixNanos>,
    pub ts_closed: Option<UnixNanos>,

    pub sequence: SequenceNumber,
}

impl Order {
    pub fn market(
        instrument_id: InstrumentId,
        side: OrderSide,
        quantity: Quantity,
        strategy_id: StrategyId,
        account_id: AccountId,
    ) -> Self {
        Self {
            id: OrderId::generate(),
            venue_order_id: None,
            instrument_id,
            strategy_id,
            account_id,
            side,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::IOC,
            price: None,
            trigger_price: None,
            quantity,
            reduce_only: false,
            post_only: false,
            state: OrderState::New,
            filled_quantity: Quantity::ZERO,
            average_fill_price: None,
            ts_created: UnixNanos::now(),
            ts_submitted: None,
            ts_accepted: None,
            ts_last_fill: None,
            ts_closed: None,
            sequence: SequenceNumber::default(),
        }
    }

    pub fn limit(
        instrument_id: InstrumentId,
        side: OrderSide,
        price: Price,
        quantity: Quantity,
        strategy_id: StrategyId,
        account_id: AccountId,
    ) -> Self {
        Self {
            id: OrderId::generate(),
            venue_order_id: None,
            instrument_id,
            strategy_id,
            account_id,
            side,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::GTC,
            price: Some(price),
            trigger_price: None,
            quantity,
            reduce_only: false,
            post_only: false,
            state: OrderState::New,
            filled_quantity: Quantity::ZERO,
            average_fill_price: None,
            ts_created: UnixNanos::now(),
            ts_submitted: None,
            ts_accepted: None,
            ts_last_fill: None,
            ts_closed: None,
            sequence: SequenceNumber::default(),
        }
    }

    pub fn remaining_quantity(&self) -> Quantity {
        self.quantity - self.filled_quantity
    }

    pub fn is_filled(&self) -> bool {
        self.state == OrderState::Filled
    }

    pub fn transition_to(
        &mut self,
        new_state: OrderState,
        ts: UnixNanos,
    ) -> Result<(), OrderError> {
        if !self.state.can_transition_to(new_state) {
            return Err(OrderError::InvalidStateTransition {
                from: self.state,
                to: new_state,
            });
        }

        self.state = new_state;

        match new_state {
            OrderState::Submitted => self.ts_submitted = Some(ts),
            OrderState::Accepted => self.ts_accepted = Some(ts),
            OrderState::Filled
            | OrderState::Canceled
            | OrderState::Rejected
            | OrderState::Expired => {
                self.ts_closed = Some(ts);
            }
            _ => {}
        }

        Ok(())
    }

    pub fn apply_fill(&mut self, fill: &Fill) -> Result<(), OrderError> {
        if fill.order_id != self.id {
            return Err(OrderError::OrderIdMismatch);
        }

        let new_filled = self.filled_quantity + fill.quantity;
        if new_filled > self.quantity {
            return Err(OrderError::OverFill);
        }

        let old_value = self.filled_quantity.to_f64()
            * self.average_fill_price.map(|p| p.to_f64()).unwrap_or(0.0);
        let fill_value = fill.quantity.to_f64() * fill.price.to_f64();
        let new_avg = (old_value + fill_value) / new_filled.to_f64();
        self.average_fill_price = Some(Price::from_f64(new_avg, 8));

        self.filled_quantity = new_filled;
        self.ts_last_fill = Some(fill.ts_event);

        if new_filled == self.quantity {
            self.transition_to(OrderState::Filled, fill.ts_event)?;
        } else if self.state == OrderState::Submitted || self.state == OrderState::Accepted {
            self.transition_to(OrderState::PartiallyFilled, fill.ts_event)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderError {
    InvalidStateTransition { from: OrderState, to: OrderState },
    OrderIdMismatch,
    OverFill,
    OrderNotFound,
    InvalidPrice,
    InvalidQuantity,
}

impl std::fmt::Display for OrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderError::InvalidStateTransition { from, to } => {
                write!(f, "Invalid state transition from {} to {}", from, to)
            }
            OrderError::OrderIdMismatch => write!(f, "Order ID mismatch"),
            OrderError::OverFill => write!(f, "Fill would exceed order quantity"),
            OrderError::OrderNotFound => write!(f, "Order not found"),
            OrderError::InvalidPrice => write!(f, "Invalid price"),
            OrderError::InvalidQuantity => write!(f, "Invalid quantity"),
        }
    }
}

impl std::error::Error for OrderError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub id: TradeId,
    pub order_id: OrderId,
    pub venue_order_id: Option<VenueOrderId>,
    pub instrument_id: InstrumentId,
    pub venue: Venue,
    pub side: OrderSide,
    pub price: Price,
    pub quantity: Quantity,
    pub commission: Money,
    pub liquidity_side: LiquiditySide,
    pub ts_event: UnixNanos,
    pub ts_recv: UnixNanos,
}

impl Fill {
    pub fn notional(&self) -> FixedPoint {
        self.price.inner() * self.quantity.inner()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LiquiditySide {
    Maker,
    Taker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PositionSide {
    Flat,
    Long,
    Short,
}

impl std::fmt::Display for PositionSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionSide::Flat => write!(f, "FLAT"),
            PositionSide::Long => write!(f, "LONG"),
            PositionSide::Short => write!(f, "SHORT"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: PositionId,
    pub instrument_id: InstrumentId,
    pub account_id: AccountId,
    pub strategy_id: StrategyId,

    pub signed_quantity: Quantity,

    pub avg_entry_price: Price,

    pub realized_pnl: Money,

    pub unrealized_pnl: Money,

    pub commission: Money,

    pub fill_count: u64,

    pub ts_opened: UnixNanos,
    pub ts_last_update: UnixNanos,
    pub ts_closed: Option<UnixNanos>,
}

impl Position {
    pub fn new(
        instrument_id: InstrumentId,
        account_id: AccountId,
        strategy_id: StrategyId,
        ts: UnixNanos,
    ) -> Self {
        Self {
            id: PositionId::generate(),
            instrument_id,
            account_id,
            strategy_id,
            signed_quantity: Quantity::ZERO,
            avg_entry_price: Price::ZERO,
            realized_pnl: Money::usd(FixedPoint::ZERO),
            unrealized_pnl: Money::usd(FixedPoint::ZERO),
            commission: Money::usd(FixedPoint::ZERO),
            fill_count: 0,
            ts_opened: ts,
            ts_last_update: ts,
            ts_closed: None,
        }
    }

    pub fn side(&self) -> PositionSide {
        if self.signed_quantity.inner().is_zero() {
            PositionSide::Flat
        } else if self.signed_quantity.inner().is_positive() {
            PositionSide::Long
        } else {
            PositionSide::Short
        }
    }

    pub fn is_flat(&self) -> bool {
        self.signed_quantity.is_zero()
    }

    pub fn quantity(&self) -> Quantity {
        self.signed_quantity.abs()
    }

    pub fn apply_fill(&mut self, fill: &Fill) {
        let fill_signed_qty = if fill.side == OrderSide::Buy {
            fill.quantity
        } else {
            -fill.quantity
        };

        let old_qty = self.signed_quantity;
        let new_qty = old_qty + fill_signed_qty;

        if !old_qty.is_zero() {
            let old_side = if old_qty.inner().is_positive() {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            };
            let is_reducing = fill.side != old_side;

            if is_reducing {
                let close_qty = fill.quantity.inner().value.min(old_qty.inner().value.abs());
                let entry_price = self.avg_entry_price.to_f64();
                let exit_price = fill.price.to_f64();
                let pnl = if old_side == OrderSide::Buy {
                    (exit_price - entry_price) * close_qty as f64 / 10_f64.powi(8)
                } else {
                    (entry_price - exit_price) * close_qty as f64 / 10_f64.powi(8)
                };
                let pnl_fp = FixedPoint::from_f64(pnl, 8);
                self.realized_pnl = Money::usd(self.realized_pnl.amount + pnl_fp);
            }
        }

        if new_qty.inner().is_positive() && fill.side == OrderSide::Buy {
            if old_qty.inner().is_positive() {
                let old_val = old_qty.to_f64() * self.avg_entry_price.to_f64();
                let new_val = fill.quantity.to_f64() * fill.price.to_f64();
                let avg = (old_val + new_val) / new_qty.to_f64();
                self.avg_entry_price = Price::from_f64(avg, 8);
            } else {
                self.avg_entry_price = fill.price;
            }
        } else if new_qty.inner().is_negative() && fill.side == OrderSide::Sell {
            if old_qty.inner().is_negative() {
                let old_val = old_qty.to_f64().abs() * self.avg_entry_price.to_f64();
                let new_val = fill.quantity.to_f64() * fill.price.to_f64();
                let avg = (old_val + new_val) / new_qty.to_f64().abs();
                self.avg_entry_price = Price::from_f64(avg, 8);
            } else {
                self.avg_entry_price = fill.price;
            }
        }

        self.signed_quantity = new_qty;
        self.commission = Money::usd(self.commission.amount + fill.commission.amount);
        self.fill_count += 1;
        self.ts_last_update = fill.ts_event;

        if self.is_flat() {
            self.ts_closed = Some(fill.ts_event);
        }
    }

    pub fn update_unrealized_pnl(&mut self, mark_price: Price) {
        if self.is_flat() {
            self.unrealized_pnl = Money::usd(FixedPoint::ZERO);
            return;
        }

        let qty = self.signed_quantity.to_f64();
        let entry = self.avg_entry_price.to_f64();
        let mark = mark_price.to_f64();

        let pnl = qty * (mark - entry);
        self.unrealized_pnl = Money::usd(FixedPoint::from_f64(pnl, 8));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentSpec {
    pub id: InstrumentId,
    pub base_currency: Currency,
    pub quote_currency: Currency,

    pub tick_size: Price,

    pub lot_size: Quantity,

    pub min_quantity: Quantity,

    pub max_quantity: Option<Quantity>,

    pub max_leverage: Option<u32>,

    pub maker_fee: FixedPoint,

    pub taker_fee: FixedPoint,

    pub is_active: bool,
}

impl InstrumentSpec {
    pub fn round_price(&self, price: Price) -> Price {
        let tick = self.tick_size.inner().value;
        let rounded = (price.inner().value / tick) * tick;
        Price::new(rounded, price.inner().scale)
    }

    pub fn round_quantity(&self, quantity: Quantity) -> Quantity {
        let lot = self.lot_size.inner().value;
        let rounded = (quantity.inner().value / lot) * lot;
        Quantity::new(rounded, quantity.inner().scale)
    }

    pub fn validate_order(&self, price: Option<Price>, quantity: Quantity) -> Result<(), String> {
        if quantity < self.min_quantity {
            return Err(format!(
                "Quantity {} below minimum {}",
                quantity, self.min_quantity
            ));
        }

        if let Some(max) = self.max_quantity {
            if quantity > max {
                return Err(format!("Quantity {} above maximum {}", quantity, max));
            }
        }

        if let Some(p) = price {
            let rounded = self.round_price(p);
            if rounded != p {
                return Err(format!(
                    "Price {} not on tick grid (tick size: {})",
                    p, self.tick_size
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeTick {
    pub instrument_id: InstrumentId,
    pub price: Price,
    pub quantity: Quantity,
    pub aggressor_side: OrderSide,
    pub trade_id: TradeId,
    pub ts_event: UnixNanos,
    pub ts_recv: UnixNanos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteTick {
    pub instrument_id: InstrumentId,
    pub bid_price: Price,
    pub bid_quantity: Quantity,
    pub ask_price: Price,
    pub ask_quantity: Quantity,
    pub ts_event: UnixNanos,
    pub ts_recv: UnixNanos,
}

impl QuoteTick {
    pub fn mid_price(&self) -> Price {
        let mid = (self.bid_price.to_f64() + self.ask_price.to_f64()) / 2.0;
        Price::from_f64(mid, 8)
    }

    pub fn spread(&self) -> Price {
        Price::new(
            self.ask_price.inner().value - self.bid_price.inner().value,
            self.ask_price.inner().scale,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookLevel {
    pub price: Price,
    pub quantity: Quantity,
    pub order_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookSnapshot {
    pub instrument_id: InstrumentId,
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
    pub sequence: u64,
    pub ts_event: UnixNanos,
    pub ts_recv: UnixNanos,
}

impl OrderBookSnapshot {
    pub fn best_bid(&self) -> Option<&BookLevel> {
        self.bids.first()
    }

    pub fn best_ask(&self) -> Option<&BookLevel> {
        self.asks.first()
    }

    pub fn mid_price(&self) -> Option<Price> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => {
                let mid = (bid.price.to_f64() + ask.price.to_f64()) / 2.0;
                Some(Price::from_f64(mid, 8))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookDelta {
    pub instrument_id: InstrumentId,
    pub action: BookAction,
    pub side: OrderSide,
    pub price: Price,
    pub quantity: Quantity,
    pub sequence: u64,
    pub ts_event: UnixNanos,
    pub ts_recv: UnixNanos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BookAction {
    Add,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar {
    pub instrument_id: InstrumentId,
    pub open: Price,
    pub high: Price,
    pub low: Price,
    pub close: Price,
    pub volume: Quantity,
    pub bar_count: u64,
    pub ts_open: UnixNanos,
    pub ts_close: UnixNanos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountState {
    pub account_id: AccountId,
    pub venue: Venue,
    pub balances: Vec<AccountBalance>,
    pub margin_used: Money,
    pub margin_available: Money,
    pub leverage: u32,
    pub ts_event: UnixNanos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    pub currency: Currency,
    pub total: FixedPoint,
    pub available: FixedPoint,
    pub locked: FixedPoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderCommand {
    Submit(SubmitOrder),
    Cancel(CancelOrder),
    Modify(ModifyOrder),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitOrder {
    pub order: Order,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrder {
    pub order_id: OrderId,
    pub venue_order_id: Option<VenueOrderId>,
    pub instrument_id: InstrumentId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifyOrder {
    pub order_id: OrderId,
    pub venue_order_id: Option<VenueOrderId>,
    pub instrument_id: InstrumentId,
    pub new_price: Option<Price>,
    pub new_quantity: Option<Quantity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderEvent {
    Submitted(OrderSubmitted),
    Accepted(OrderAccepted),
    Rejected(OrderRejected),
    Filled(OrderFilled),
    PartiallyFilled(OrderPartiallyFilled),
    Canceled(OrderCanceled),
    CancelRejected(OrderCancelRejected),
    Modified(OrderModified),
    ModifyRejected(OrderModifyRejected),
    Expired(OrderExpired),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderSubmitted {
    pub order_id: OrderId,
    pub ts_event: UnixNanos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAccepted {
    pub order_id: OrderId,
    pub venue_order_id: VenueOrderId,
    pub ts_event: UnixNanos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRejected {
    pub order_id: OrderId,
    pub reason: String,
    pub ts_event: UnixNanos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFilled {
    pub order_id: OrderId,
    pub fill: Fill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPartiallyFilled {
    pub order_id: OrderId,
    pub fill: Fill,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCanceled {
    pub order_id: OrderId,
    pub venue_order_id: Option<VenueOrderId>,
    pub ts_event: UnixNanos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCancelRejected {
    pub order_id: OrderId,
    pub reason: String,
    pub ts_event: UnixNanos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderModified {
    pub order_id: OrderId,
    pub venue_order_id: VenueOrderId,
    pub new_price: Option<Price>,
    pub new_quantity: Option<Quantity>,
    pub ts_event: UnixNanos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderModifyRejected {
    pub order_id: OrderId,
    pub reason: String,
    pub ts_event: UnixNanos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderExpired {
    pub order_id: OrderId,
    pub ts_event: UnixNanos,
}

#[cfg(test)]
mod tests {
    use neleus_core_types::InstrumentType;

    use super::*;

    #[test]
    fn test_order_state_transitions() {
        assert!(OrderState::New.can_transition_to(OrderState::Submitted));
        assert!(OrderState::Submitted.can_transition_to(OrderState::Accepted));
        assert!(OrderState::Accepted.can_transition_to(OrderState::Filled));
        assert!(!OrderState::Filled.can_transition_to(OrderState::Canceled));
        assert!(!OrderState::New.can_transition_to(OrderState::Filled));
    }

    #[test]
    fn test_order_fill() {
        let instrument_id = InstrumentId::new(Venue::Simulated, "BTC", InstrumentType::Perp);
        let mut order = Order::limit(
            instrument_id.clone(),
            OrderSide::Buy,
            Price::from_f64(50000.0, 8),
            Quantity::from_f64(1.0, 8),
            StrategyId::new("test"),
            AccountId::new("acc1"),
        );

        order
            .transition_to(OrderState::Submitted, UnixNanos::now())
            .unwrap();
        order
            .transition_to(OrderState::Accepted, UnixNanos::now())
            .unwrap();

        let fill = Fill {
            id: TradeId::generate(),
            order_id: order.id.clone(),
            venue_order_id: None,
            instrument_id,
            venue: Venue::Simulated,
            side: OrderSide::Buy,
            price: Price::from_f64(50000.0, 8),
            quantity: Quantity::from_f64(0.5, 8),
            commission: Money::usd(FixedPoint::from_f64(0.50, 8)),
            liquidity_side: LiquiditySide::Taker,
            ts_event: UnixNanos::now(),
            ts_recv: UnixNanos::now(),
        };

        order.apply_fill(&fill).unwrap();
        assert_eq!(order.state, OrderState::PartiallyFilled);
        assert_eq!(order.filled_quantity.to_f64(), 0.5);
    }

    #[test]
    fn test_position_pnl() {
        let instrument_id = InstrumentId::new(Venue::Simulated, "ETH", InstrumentType::Perp);
        let mut position = Position::new(
            instrument_id.clone(),
            AccountId::new("acc1"),
            StrategyId::new("test"),
            UnixNanos::now(),
        );

        let fill1 = Fill {
            id: TradeId::generate(),
            order_id: OrderId::generate(),
            venue_order_id: None,
            instrument_id: instrument_id.clone(),
            venue: Venue::Simulated,
            side: OrderSide::Buy,
            price: Price::from_f64(2000.0, 8),
            quantity: Quantity::from_f64(10.0, 8),
            commission: Money::usd(FixedPoint::from_f64(2.0, 8)),
            liquidity_side: LiquiditySide::Taker,
            ts_event: UnixNanos::now(),
            ts_recv: UnixNanos::now(),
        };
        position.apply_fill(&fill1);

        assert_eq!(position.side(), PositionSide::Long);
        assert_eq!(position.quantity().to_f64(), 10.0);

        position.update_unrealized_pnl(Price::from_f64(2100.0, 8));
        assert!(position.unrealized_pnl.amount.is_positive());
    }
}
