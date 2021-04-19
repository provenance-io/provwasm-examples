use crate::state::{BuyOrder, SellOrder};
use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub buy_denom: String,
    pub price: Uint128, // Price per 1 hash
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Buy { id: String },  // Will require sent funds to be buy_denom (ie stablecoin)
    Sell { id: String }, // Will require sent funds to be sell_denom (ie nhash)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetBuyOrders {},
    GetSellOrders {},
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
