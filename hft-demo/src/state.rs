use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

pub static CONFIG_KEY: &[u8] = b"config";

pub static TRADER_KEY: &[u8] = b"trader";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub contract_admin: HumanAddr, // Ensures only sender from contract init can call handle.
    pub security: String,          // The denom of the stock pool marker.
    pub stablecoin: String,        // The denom of the loan pool marker.
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TraderState {
    pub loan_cap: Uint128, // The max amount of stablecoin that can be loaned to this trader
    pub loans: Uint128,    // The current amount of stablecoin loaned to this trader
}

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn trader_bucket(storage: &mut dyn Storage) -> Bucket<TraderState> {
    bucket(storage, TRADER_KEY)
}

pub fn trader_bucket_read(storage: &dyn Storage) -> ReadonlyBucket<TraderState> {
    bucket_read(storage, TRADER_KEY)
}
