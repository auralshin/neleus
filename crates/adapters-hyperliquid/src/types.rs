use neleus_core_types::OrderSide;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct WsSubscription {
    pub subscription_type: WsSubscriptionType,
    pub instrument: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsSubscriptionType {
    AllMids,
    L2Book,
    Trades,
    Candle,
    UserFills,
    UserOrders,
    UserFunding,
    Notification,
}

impl WsSubscription {
    pub fn to_json(&self) -> String {
        match &self.subscription_type {
            WsSubscriptionType::AllMids => {
                r#"{"method":"subscribe","subscription":{"type":"allMids"}}"#.to_string()
            }
            WsSubscriptionType::L2Book => {
                if let Some(coin) = &self.instrument {
                    format!(
                        r#"{{"method":"subscribe","subscription":{{"type":"l2Book","coin":"{}"}}}}"#,
                        coin
                    )
                } else {
                    String::new()
                }
            }
            WsSubscriptionType::Trades => {
                if let Some(coin) = &self.instrument {
                    format!(
                        r#"{{"method":"subscribe","subscription":{{"type":"trades","coin":"{}"}}}}"#,
                        coin
                    )
                } else {
                    String::new()
                }
            }
            WsSubscriptionType::Candle => {
                if let Some(coin) = &self.instrument {
                    format!(
                        r#"{{"method":"subscribe","subscription":{{"type":"candle","coin":"{}","interval":"1m"}}}}"#,
                        coin
                    )
                } else {
                    String::new()
                }
            }
            WsSubscriptionType::UserFills => {
                r#"{"method":"subscribe","subscription":{"type":"userFills"}}"#.to_string()
            }
            WsSubscriptionType::UserOrders => {
                r#"{"method":"subscribe","subscription":{"type":"userOrders"}}"#.to_string()
            }
            WsSubscriptionType::UserFunding => {
                r#"{"method":"subscribe","subscription":{"type":"userFunding"}}"#.to_string()
            }
            WsSubscriptionType::Notification => {
                r#"{"method":"subscribe","subscription":{"type":"notification"}}"#.to_string()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct HyperliquidTrade {
    pub coin: String,
    pub side: OrderSide,
    pub price: f64,
    pub size: f64,
    pub timestamp: u64,
    pub trade_id: u64,
}

#[derive(Debug, Clone)]
pub struct HyperliquidBookLevel {
    pub price: f64,
    pub size: f64,
    pub num_orders: u32,
}

#[derive(Debug, Clone)]
pub struct HyperliquidBook {
    pub coin: String,
    pub bids: Vec<HyperliquidBookLevel>,
    pub asks: Vec<HyperliquidBookLevel>,
    pub timestamp: u64,
    pub is_snapshot: bool,
}

#[derive(Debug, Clone)]
pub struct HyperliquidUserFill {
    pub coin: String,
    pub order_id: u64,
    pub side: OrderSide,
    pub price: f64,
    pub size: f64,
    pub fee: f64,
    pub timestamp: u64,
    pub crossed: bool,
}

#[derive(Debug, Clone)]
pub struct HyperliquidUserOrder {
    pub coin: String,
    pub order_id: u64,
    pub client_order_id: Option<String>,
    pub side: OrderSide,
    pub order_type: HyperliquidOrderType,
    pub price: Option<f64>,
    pub size: f64,
    pub filled_size: f64,
    pub status: HyperliquidOrderStatus,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperliquidOrderType {
    Limit,
    Market,
    StopMarket,
    StopLimit,
    TakeProfit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperliquidOrderStatus {
    Open,
    Filled,
    Canceled,
    Rejected,
    Triggered,
}

#[derive(Debug, Clone)]
pub struct HyperliquidOrderRequest {
    pub coin: String,
    pub is_buy: bool,
    pub limit_px: f64,
    pub sz: f64,
    pub reduce_only: bool,
    pub order_type: HyperliquidOrderTypeRequest,
    pub cloid: Option<String>,
}

#[derive(Debug, Clone)]
pub enum HyperliquidOrderTypeRequest {
    Limit {
        tif: HyperliquidTif,
    },
    Trigger {
        trigger_px: f64,
        is_market: bool,
        tpsl: TpSlType,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperliquidTif {
    Gtc,
    Ioc,
    Alo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpSlType {
    Tp,
    Sl,
}

#[derive(Debug, Clone)]
pub struct HyperliquidInstrumentInfo {
    pub coin: String,
    pub sz_decimals: u32,
    pub max_leverage: u32,
    pub only_isolated: bool,
    pub funding_rate: f64,
    pub open_interest: f64,
    pub prev_day_px: f64,
    pub day_ntl_vlm: f64,
    pub premium: f64,
    pub oracle_px: f64,
    pub mark_px: f64,
}

// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "channel")]
pub enum HyperliquidWsMessage {
    #[serde(rename = "allMids")]
    AllMids { data: AllMidsData },
    #[serde(rename = "l2Book")]
    L2Book { data: L2BookData },
    #[serde(rename = "trades")]
    Trades { data: Vec<TradeData> },
    #[serde(rename = "user")]
    User { data: UserData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllMidsData {
    pub mids: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2BookData {
    pub coin: String,
    pub time: u64,
    pub levels: Vec<Vec<PriceLevel>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub px: String,
    pub sz: String,
    pub n: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeData {
    pub coin: String,
    pub side: String,
    pub px: String,
    pub sz: String,
    pub time: u64,
    pub tid: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserData {
    #[serde(rename = "fill")]
    Fill { data: FillData },
    #[serde(rename = "order")]
    Order { data: OrderData },
    #[serde(rename = "notification")]
    Notification { data: NotificationData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillData {
    pub coin: String,
    pub px: String,
    pub sz: String,
    pub side: String,
    pub time: u64,
    pub oid: u64,
    pub tid: u64,
    pub fee: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderData {
    pub coin: String,
    pub side: String,
    pub limit_px: String,
    pub sz: String,
    pub oid: u64,
    pub timestamp: u64,
    pub order_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationData {
    pub notification: String,
}

// Order execution types
#[derive(Debug, Clone)]
pub struct PlaceOrderAction {
    pub nonce: u64,
    pub orders: Vec<HyperliquidOrderRequest>,
    pub grouping: OrderGrouping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderGrouping {
    Na,
    NormalTpsl,
    PositionTpsl,
}

#[derive(Debug, Clone)]
pub struct CancelOrderAction {
    pub nonce: u64,
    pub cancels: Vec<CancelRequest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CancelRequest {
    pub coin: String,
    #[serde(rename = "oid")]
    pub order_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceOrderResponse {
    pub status: String,
    #[serde(default)]
    pub response: Option<OrderResponseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponseData {
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: Option<OrderResponseStatuses>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponseStatuses {
    pub statuses: Vec<OrderStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrderStatus {
    Resting { resting: OrderIdResponse },
    Filled { filled: OrderIdResponse },
    Error { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderIdResponse {
    pub oid: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelOrderResponse {
    pub status: String,
    #[serde(default)]
    pub response: Option<CancelResponseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelResponseData {
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: Option<CancelResponseStatuses>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelResponseStatuses {
    pub statuses: Vec<String>,
}
