use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub exchange: HumanAddr,   // The address of the exchange sending settlements
    pub contract_name: String, // A name for the contract instance.
    pub denoms: Vec<String>,   // Restrict settlements to these denominations.
    pub attrs: Vec<String>,    // The attributes required for transfer (empty means none required).
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Settlement { to: HumanAddr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}
