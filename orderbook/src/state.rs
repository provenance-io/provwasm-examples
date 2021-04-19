use cosmwasm_std::{Coin, HumanAddr, Uint128};
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
    pub contract_admin: HumanAddr,
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
    pub amount: Coin,
}

/// Persisted sell when a real-time buy matches are not found.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SellOrder {
    pub id: String,
    pub price: Uint128,
    pub ts: u64,
    pub seller: HumanAddr,
    pub amount: Coin,
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
    bucket(storage, BUY_KEY)
}

pub fn sell_orders_read(storage: &dyn Storage) -> ReadonlyBucket<SellOrder> {
    bucket_read(storage, BUY_KEY)
}
