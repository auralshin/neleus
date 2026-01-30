use neleus_core_types::OrderSide;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsSubscriptionType {
    OrderBook,
    Trades,
    Orders,
    Fills,
    Account,
}

#[derive(Debug, Clone)]
pub struct WsSubscription {
    pub subscription_type: WsSubscriptionType,
    pub market_id: Option<u32>,
}

impl WsSubscription {
    pub fn to_message(&self) -> LighterWsMessage {
        match &self.subscription_type {
            WsSubscriptionType::OrderBook => LighterWsMessage::Subscribe {
                channel: "orderbook".to_string(),
                market_id: self.market_id,
            },
            WsSubscriptionType::Trades => LighterWsMessage::Subscribe {
                channel: "trades".to_string(),
                market_id: self.market_id,
            },
            WsSubscriptionType::Orders => LighterWsMessage::Subscribe {
                channel: "orders".to_string(),
                market_id: None,
            },
            WsSubscriptionType::Fills => LighterWsMessage::Subscribe {
                channel: "fills".to_string(),
                market_id: None,
            },
            WsSubscriptionType::Account => LighterWsMessage::Subscribe {
                channel: "account".to_string(),
                market_id: None,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum LighterWsMessage {
    Subscribe {
        channel: String,
        market_id: Option<u32>,
    },
    Unsubscribe {
        channel: String,
        market_id: Option<u32>,
    },
    Ping,
    Pong,
}

impl LighterWsMessage {
    pub fn to_json(&self) -> String {
        match self {
            LighterWsMessage::Subscribe { channel, market_id } => {
                if let Some(id) = market_id {
                    format!(
                        r#"{{"op":"subscribe","channel":"{}","market_id":{}}}"#,
                        channel, id
                    )
                } else {
                    format!(r#"{{"op":"subscribe","channel":"{}"}}"#, channel)
                }
            }
            LighterWsMessage::Unsubscribe { channel, market_id } => {
                if let Some(id) = market_id {
                    format!(
                        r#"{{"op":"unsubscribe","channel":"{}","market_id":{}}}"#,
                        channel, id
                    )
                } else {
                    format!(r#"{{"op":"unsubscribe","channel":"{}"}}"#, channel)
                }
            }
            LighterWsMessage::Ping => r#"{"op":"ping"}"#.to_string(),
            LighterWsMessage::Pong => r#"{"op":"pong"}"#.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LighterMarketInfo {
    pub market_id: u32,
    pub symbol: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub price_decimals: u32,
    pub quantity_decimals: u32,
    pub min_order_size: f64,
    pub tick_size: f64,
    pub maker_fee: f64,
    pub taker_fee: f64,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct LighterTrade {
    pub market_id: u32,
    pub trade_id: u64,
    pub price: f64,
    pub quantity: f64,
    pub side: OrderSide,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct LighterBookLevel {
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone)]
pub struct LighterOrderBook {
    pub market_id: u32,
    pub bids: Vec<LighterBookLevel>,
    pub asks: Vec<LighterBookLevel>,
    pub timestamp: u64,
    pub sequence: u64,
}

#[derive(Debug, Clone)]
pub struct LighterUserOrder {
    pub order_id: String,
    pub market_id: u32,
    pub side: OrderSide,
    pub order_type: LighterOrderType,
    pub price: f64,
    pub quantity: f64,
    pub filled_quantity: f64,
    pub status: LighterOrderStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LighterOrderType {
    Limit,
    Market,
    LimitPostOnly,
    LimitIoc,
    LimitFok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LighterOrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
}

#[derive(Debug, Clone)]
pub struct LighterUserFill {
    pub fill_id: String,
    pub order_id: String,
    pub market_id: u32,
    pub side: OrderSide,
    pub price: f64,
    pub quantity: f64,
    pub fee: f64,
    pub is_maker: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LighterOrderRequest {
    pub market_id: u32,
    pub side: OrderSide,
    pub order_type: LighterOrderType,
    pub price: f64,
    pub quantity: f64,
    pub client_order_id: Option<String>,
    pub reduce_only: bool,
}

#[derive(Debug, Clone)]
pub struct LighterCancelRequest {
    pub order_id: String,
    pub market_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LighterMarketDataMessage {
    #[serde(rename = "orderbook")]
    OrderBook { data: OrderBookData },
    #[serde(rename = "trade")]
    Trade { data: LighterTradeData },
    #[serde(rename = "order")]
    Order { data: LighterOrderData },
    #[serde(rename = "fill")]
    Fill { data: LighterFillData },
    #[serde(rename = "account")]
    Account { data: AccountData },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookData {
    pub market_id: u32,
    pub timestamp: u64,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LighterTradeData {
    pub market_id: u32,
    pub trade_id: u64,
    pub price: String,
    pub size: String,
    pub side: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LighterOrderData {
    pub order_id: String,
    pub market_id: u32,
    pub user_address: String,
    pub price: String,
    pub size: String,
    pub side: String,
    pub order_type: String,
    pub status: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LighterFillData {
    pub fill_id: u64,
    pub order_id: String,
    pub market_id: u32,
    pub price: String,
    pub size: String,
    pub side: String,
    pub fee: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountData {
    pub address: String,
    pub balances: HashMap<String, String>,
}
