use cosmwasm_std::Uint128;
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
    AddTrader { address: String }, // Sets loan cap based on stablecoin balance.
    BuyStock { amount: Uint128 },  // The shares to buy
    SellStock { amount: Uint128 }, // The shares to sell
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetTraderState { address: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TraderStateResponse {
    pub security: Uint128,
    pub stablecoin: Uint128,
    pub loans: Uint128,
    pub loan_cap: Uint128,
}

/// Migrate the contract.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}
