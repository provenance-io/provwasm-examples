use crate::state::{AskOrder, BidOrder};
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub bid_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Bid { id: String, price: Uint128 }, // Number of stablecoins offered for 1 hash
    Ask { id: String, price: Uint128 }, // Number of stablecoins requested for 1 hash
    Match {},                           // Match each ask to >= 1 bids
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBidOrders {},
    GetAskOrders {},
    GetOrderbook {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct BidOrders {
    pub bid_orders: Vec<BidOrder>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct AskOrders {
    pub ask_orders: Vec<AskOrder>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Orderbook {
    pub bid_orders: Vec<BidOrder>,
    pub ask_orders: Vec<AskOrder>,
}
