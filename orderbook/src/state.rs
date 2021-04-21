use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Storage;
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
pub static CONFIG_KEY: &[u8] = b"config";
pub static BUY_KEY: &[u8] = b"buy";
pub static SELL_KEY: &[u8] = b"sell";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub sell_denom: String,
    pub buy_denom: String,
}

/// Persisted buy when a real-time sell match is not found.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BuyOrder {
    pub id: String,
    pub price: Uint128,
    pub ts: u64,
    pub buyer: HumanAddr,
    pub funds: Uint128,       // The stablecoin available for transfer
    pub outstanding: Uint128, // The outstanding proceeds for the buy
}

/// Persisted sell when a real-time buy matches are not found.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SellOrder {
    pub id: String,
    pub price: Uint128,
    pub ts: u64,
    pub seller: HumanAddr,
    pub funds: Uint128,       // The nhash available for transfer
    pub outstanding: Uint128, // The outstanding proceeds for the sell
}

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn buy_orders(storage: &mut dyn Storage) -> Bucket<BuyOrder> {
    bucket(storage, BUY_KEY)
}

pub fn buy_orders_read(storage: &dyn Storage) -> ReadonlyBucket<BuyOrder> {
    bucket_read(storage, BUY_KEY)
}

pub fn sell_orders(storage: &mut dyn Storage) -> Bucket<SellOrder> {
    bucket(storage, SELL_KEY)
}

pub fn sell_orders_read(storage: &dyn Storage) -> ReadonlyBucket<SellOrder> {
    bucket_read(storage, SELL_KEY)
}
