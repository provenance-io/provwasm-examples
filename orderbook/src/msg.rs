use crate::state::{BuyOrder, SellOrder};
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub buy_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Buy { id: String, price: Uint128 }, // Number of stablecoins offered for 1 hash
    Sell { id: String, price: Uint128 }, // Number of stablecoins requested for 1 hash
    Match {},                           // Match 1 sell to >= 1 buys
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBuyOrders {},
    GetSellOrders {},
    GetOrderbook {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct BuyOrders {
    pub buy_orders: Vec<BuyOrder>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SellOrders {
    pub sell_orders: Vec<SellOrder>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Orderbook {
    pub buy_orders: Vec<BuyOrder>,
    pub sell_orders: Vec<SellOrder>,
}
