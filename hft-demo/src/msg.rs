use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    pub security: String,   // The denom of the stock pool marker
    pub stablecoin: String, // The denom of the loan pool marker
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddTrader { address: HumanAddr }, // Sets loan cap based on stablecoin balance.
    BuyStock { amount: Uint128 },     // The shares to buy
    SellStock { amount: Uint128 },    // The shares to sell
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetTraderState { address: HumanAddr },
    GetDenoms {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Denoms {
    pub security: String,
    pub stablecoin: String,
}
