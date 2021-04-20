use cosmwasm_std::{
    coin, to_binary, Decimal, Deps, DepsMut, Env, MessageInfo, Order, QueryResponse, Response,
    StdResult, Uint128,
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
    _info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    // Create and store config state.
    let state = State {
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

    // :maths: - stablecoin sent * (1000000000 nhash / price stablecoins) = nhash received
    // Just assume no rounding issues for now.
    // TODO: use/enforce `price_precision` and `quantity_increment` here?
    let amt = funds.amount * Decimal::from_ratio(1000000000u128, price.u128());
    let recv_amount = coin(amt.u128(), "nhash");

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
            send_amount: funds, // Send this amount to seller(s)
            recv_amount,        // Receive this amount from seller(s)
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

    // :maths: - nhash sent * (price stablecoins / 1000000000 nhash) = stablecoins received
    // Just assume no rounding issues for now.
    // TODO: use/enforce `price_precision` and `quantity_increment` here?
    let amt = funds.amount * Decimal::from_ratio(price.u128(), 1000000000u128);
    let recv_amount = coin(amt.u128(), state.buy_denom);

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
            send_amount: funds, // Send this to buyer(s)
            recv_amount,        // Receive this from buyer(s)
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
    // Query sorted buy orders, checking for errors
    let buy_orders = get_buy_orders(deps)?;
    // Serialize and return
    let bin = to_binary(&BuyOrders { buy_orders })?;
    Ok(bin)
}

// Read all buy orders into memory then sort by price, timestamp.
fn get_buy_orders(deps: Deps) -> Result<Vec<BuyOrder>, ContractError> {
    // Read all buy orders
    let buy_orders: StdResult<Vec<_>> = buy_orders_read(deps.storage)
        .range(None, None, Order::Ascending)
        .map(|item| {
            let (_, buy_order) = item?;
            Ok(buy_order)
        })
        .collect();

    // Check for error
    let mut buy_orders = buy_orders?;

    // Sort by price, then time.
    buy_orders.sort_by(|a, b| {
        if a.price != b.price {
            b.price.cmp(&a.price) // flip comparison for best price first
        } else {
            a.ts.cmp(&b.ts)
        }
    });

    // Return sorted in price-time order
    Ok(buy_orders)
}

// Read all sell orders into memory, sort by amount/ts, then serialize to JSON.
fn try_get_sell_orders(deps: Deps) -> Result<QueryResponse, ContractError> {
    // Query sorted sell orders, checking for errors
    let sell_orders = get_sell_orders(deps)?;
    // Serialize and return
    let bin = to_binary(&SellOrders { sell_orders })?;
    Ok(bin)
}

// Read all sell orders into memory then sort by price, timestamp.
fn get_sell_orders(deps: Deps) -> Result<Vec<SellOrder>, ContractError> {
    // Read all sell orders
    let sell_orders: StdResult<Vec<_>> = sell_orders_read(deps.storage)
        .range(None, None, Order::Ascending)
        .map(|item| {
            let (_, sell_order) = item?;
            Ok(sell_order)
        })
        .collect();

    // Check for error
    let mut sell_orders = sell_orders?;

    // Sort by price, then time.
    sell_orders.sort_by(|a, b| {
        if a.price != b.price {
            b.price.cmp(&a.price) // flip comparison for best price first
        } else {
            a.ts.cmp(&b.ts)
        }
    });

    // Return sorted in price-time order
    Ok(sell_orders)
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
