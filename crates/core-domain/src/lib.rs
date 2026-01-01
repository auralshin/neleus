use neleus_core_types::{InstrumentId, OrderId, Price, Quantity, StrategyId, TimeStamp, VenueOrderId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderType {
    Market,
    Limit,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Order {
    pub id: OrderId,
    pub venue_order_id: Option<VenueOrderId>,
    pub instrument_id: InstrumentId,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<Price>,
    pub quantity: Quantity,
    pub state: OrderState,
    pub strategy_id: StrategyId,
    pub ts_event: TimeStamp,
}

impl Order {
    pub fn new(
        id: OrderId,
        instrument_id: InstrumentId,
        side: OrderSide,
        order_type: OrderType,
        price: Option<Price>,
        quantity: Quantity,
        strategy_id: StrategyId,
        ts_event: TimeStamp,
    ) -> Self {
        Self {
            id,
            venue_order_id: None,
            instrument_id,
            side,
            order_type,
            price,
            quantity,
            state: OrderState::New,
            strategy_id,
            ts_event,
        }
    }
}
