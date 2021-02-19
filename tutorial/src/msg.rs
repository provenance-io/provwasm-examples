use cosmwasm_std::{Decimal, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::State;

/// A message sent to initialize the contract state.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub contract_name: String,
    pub purchase_denom: String,
    pub merchant_address: HumanAddr,
    pub fee_percent: Decimal,
}

/// A message sent to transfer funds and collect fees for a purchase.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Purchase { id: String },
}

/// A message sent to query contract config state.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    QueryRequest {},
}

/// A type alias for contract state.
pub type QueryResponse = State;

/// Migrate the contract, setting a new fee percentage.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {
    pub new_fee_percent: Decimal,
}
