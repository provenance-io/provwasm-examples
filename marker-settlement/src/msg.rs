use cosmwasm_std::{Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub exchange: HumanAddr,   // The exchange sending settlements
    pub contract_name: String, // Give the instance a name
    pub denoms: Vec<String>,   // Restrict settlements to specific denominations.
    pub attrs: Vec<String>,    // The attributes required for transfer (empty means none required).
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Settlement {
        coin: Coin,
        to: HumanAddr,
        from: HumanAddr,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}
