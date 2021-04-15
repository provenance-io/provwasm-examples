use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{BuyOrder, SellOrder};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {}

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
