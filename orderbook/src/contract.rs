use cosmwasm_std::{
    to_binary, Deps, DepsMut, Env, MessageInfo, Order, QueryResponse, Response, StdResult, Uint128,
    KV,
};

use crate::error::ContractError;
use crate::msg::{BuyOrders, ExecuteMsg, InitMsg, QueryMsg, SellOrders};
use crate::state::{
    buy_orders, buy_orders_read, config, config_read, sell_orders, sell_orders_read, BuyOrder,
    SellOrder, State,
};

/// Initialize and save config state.
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    // Create and store config state.
    let state = State {
        contract_admin: info.sender,
        sell_denom: "nhash".into(), // Force nano-hash
        buy_denom: msg.buy_denom,
    };
    config(deps.storage).save(&state)?;
    Ok(Response::default())
}

/// Execute a buy or sell with automatic matching.
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Buy { id, price } => try_buy(deps, env, info, id, price),
        ExecuteMsg::Sell { id, price } => try_sell(deps, env, info, id, price),
    }
}

// Look for a sell order that will satisfy a buy. If found, settle immediately. If not found,
// persist the buy order for later matching.
fn try_buy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
    price: Uint128,
) -> Result<Response, ContractError> {
    // Ensure price is non-zero
    if price.is_zero() {
        return Err(ContractError::InvalidPrice {
            message: "price must be > 0".into(),
        });
    }

    // Ensure the correct funds where sent
    if info.funds.len() != 1 {
        return Err(ContractError::InvalidFunds {
            message: "no buy funds provided".into(),
        });
    }
    let funds = info.funds[0].clone();

    // Load config state
    let state = config_read(deps.storage).load()?;

    // Ensure the funds are valid
    if funds.amount.is_zero() {
        return Err(ContractError::InvalidFunds {
            message: "buy amount must be > 0".into(),
        });
    }
    if funds.denom != state.buy_denom {
        return Err(ContractError::InvalidFunds {
            message: format!(
                "invalid buy denom: got {}, require {}",
                funds.denom, state.buy_denom
            ),
        });
    }

    // Ensure an order with the given ID doesn't already exist.
    let order_key = id.as_bytes();
    let mut book = buy_orders(deps.storage);
    if book.may_load(&order_key)?.is_some() {
        return Err(ContractError::DuplicateBuy { id: id.clone() });
    }

    // Persist buy order
    book.save(
        &order_key,
        &BuyOrder {
            id: id.clone(),
            price,
            ts: env.block.time,
            buyer: info.sender,
            amount: funds,
        },
    )?;

    // Create response and add ID to outgoing SC `wasm` event
    let mut res = Response::new();
    res.add_attribute("action", "orderbook.buy");
    res.add_attribute("id", id);
    Ok(res)
}

fn try_sell(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
    price: Uint128,
) -> Result<Response, ContractError> {
    // Ensure price is non-zero
    if price.is_zero() {
        return Err(ContractError::InvalidPrice {
            message: "price must be > 0".into(),
        });
    }

    // Ensure the correct number of funds where sent.
    if info.funds.len() != 1 {
        return Err(ContractError::InvalidFunds {
            message: "no sell funds provided".into(),
        });
    }
    let funds = info.funds[0].clone();

    // Load config state
    let state = config_read(deps.storage).load()?;

    // Ensure the funds are valid (ie at least 1 hash was sent)
    if funds.amount < Uint128(1000000000) {
        return Err(ContractError::InvalidFunds {
            message: format!(
                "sell amount must be >= 1000000000nhash: got {}",
                funds.amount
            ),
        });
    }
    if funds.denom != state.sell_denom {
        return Err(ContractError::InvalidFunds {
            message: format!(
                "invalid sell denom: got {}, require {}",
                funds.denom, state.buy_denom
            ),
        });
    }

    // Ensure an order with the given ID doesn't already exist.
    let order_key = id.as_bytes();
    let mut sell_book = sell_orders(deps.storage);
    if sell_book.may_load(&order_key)?.is_some() {
        return Err(ContractError::DuplicateSell { id: id.clone() });
    }

    // Persist sell order
    sell_book.save(
        &order_key,
        &SellOrder {
            id: id.clone(),
            price,
            ts: env.block.time,
            seller: info.sender,
            amount: funds,
        },
    )?;

    // Create response and add ID to outgoing SC `wasm` event
    let mut res = Response::new();
    res.add_attribute("action", "orderbook.sell");
    res.add_attribute("id", id);
    Ok(res)
}

/// Query does nothing
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    match msg {
        QueryMsg::GetBuyOrders {} => try_get_buy_orders(deps),
        QueryMsg::GetSellOrders {} => try_get_sell_orders(deps),
    }
}

// Read all buy orders into memory, sort by amount/ts, then serialize to JSON.
fn try_get_buy_orders(deps: Deps) -> Result<QueryResponse, ContractError> {
    // Read all orders
    let res: StdResult<Vec<KV<BuyOrder>>> = buy_orders_read(deps.storage)
        .range(None, None, Order::Ascending)
        .collect();
    let mut buy_orders: Vec<BuyOrder> = res?.into_iter().map(|(_, v)| v).collect();

    // Sort by price, then time.
    buy_orders.sort_by(|a, b| {
        if a.price != b.price {
            b.price.cmp(&a.price)
        } else {
            a.ts.cmp(&b.ts)
        }
    });

    // Serialize and return
    let bin = to_binary(&BuyOrders { buy_orders })?;
    Ok(bin)
}

// Read all sell orders into memory, sort by amount/ts, then serialize to JSON.
fn try_get_sell_orders(deps: Deps) -> Result<QueryResponse, ContractError> {
    // Read all orders
    let res: StdResult<Vec<KV<SellOrder>>> = sell_orders_read(deps.storage)
        .range(None, None, Order::Ascending)
        .collect();
    let mut sell_orders: Vec<SellOrder> = res?.into_iter().map(|(_, v)| v).collect();

    // Sort by price, then time.
    sell_orders.sort_by(|a, b| {
        if a.price != b.price {
            b.price.cmp(&a.price)
        } else {
            a.ts.cmp(&b.ts)
        }
    });

    // Serialize and return
    let bin = to_binary(&SellOrders { sell_orders })?;
    Ok(bin)
}

#[cfg(test)]
mod tests {
    //use super::*;
    //use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    //use cosmwasm_std::{coins, from_binary};

    #[test]
    fn valid_init() {
        todo!()
    }
}
