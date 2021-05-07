use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Storage;
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
pub static CONFIG_KEY: &[u8] = b"config";
pub static BID_KEY: &[u8] = b"bid";
pub static ASK_KEY: &[u8] = b"ask";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub ask_denom: String,
    pub ask_increment: Uint128,
    pub bid_denom: String,
    pub contract_admin: Addr,
}

/// Persisted bid order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BidOrder {
    pub id: String,
    pub price: Uint128,
    pub ts: u64,
    pub bidder: Addr,
    pub funds: Uint128, // The stablecoin available for transfer
    pub funds_denom: String,
    pub proceeds: Uint128, // The proceeds for the bid
}

impl BidOrder {
    pub fn is_closed(&self) -> bool {
        self.proceeds.is_zero() && self.funds.is_zero()
    }
}

/// Persisted ask order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AskOrder {
    pub id: String,
    pub price: Uint128,
    pub ts: u64,
    pub asker: Addr,
    pub funds: Uint128, // The nhash available for transfer
    pub funds_denom: String,
    pub proceeds: Uint128, // The proceeds for the ask
}

impl AskOrder {
    pub fn is_closed(&self) -> bool {
        self.proceeds.is_zero() && self.funds.is_zero()
    }
}

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

pub fn bid_orders(storage: &mut dyn Storage) -> Bucket<BidOrder> {
    bucket(storage, BID_KEY)
}

pub fn bid_orders_read(storage: &dyn Storage) -> ReadonlyBucket<BidOrder> {
    bucket_read(storage, BID_KEY)
}

pub fn ask_orders(storage: &mut dyn Storage) -> Bucket<AskOrder> {
    bucket(storage, ASK_KEY)
}

pub fn ask_orders_read(storage: &dyn Storage) -> ReadonlyBucket<AskOrder> {
    bucket_read(storage, ASK_KEY)
}
