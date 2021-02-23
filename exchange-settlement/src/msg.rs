use cosmwasm_std::{Coin, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub contract_name: String, // A name for the exchange contract instance.
    pub bank_settlement_name: String, // The bound name of the bank settlement instance.
    pub marker_settlement_name: String, // The bound name of the marker settlement instance.
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    // Ususally, bids/asks would be stored and referenced by ID, but to keep things simple,
    // we pass them in.
    Settlement {
        asker: HumanAddr,
        ask: Coin,
        bidder: HumanAddr,
        bid: Coin,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}
