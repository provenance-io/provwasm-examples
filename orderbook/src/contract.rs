use cosmwasm_std::{
    coin, to_binary, BankMsg, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Order,
    QueryResponse, Response, StdResult, Storage, Uint128,
};

use crate::error::ContractError;
use crate::msg::{BuyOrders, ExecuteMsg, InitMsg, QueryMsg, SellOrders};
use crate::state::{
    buy_orders, buy_orders_read, config, config_read, sell_orders, sell_orders_read, BuyOrder,
    SellOrder, State,
};

use std::cmp::Ordering;

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
        ExecuteMsg::Step {} => try_step(deps),
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

    // Just assume no rounding issues for now.
    let outstanding = funds.amount * Decimal::from_ratio(1000000000u128, price.u128());

    // Validate that buy proceeds are in 1hash increments.
    if outstanding.u128() % 1000000000u128 != 0u128 {
        return Err(ContractError::InvalidFunds {
            message: "funds must yield a buy amount in 1hash increments".into(),
        });
    }

    // Persist buy order
    book.save(
        &order_key,
        &BuyOrder {
            id: id.clone(),
            price,
            ts: env.block.time,
            buyer: info.sender,
            funds: funds.amount,
            outstanding,
            denom: state.buy_denom,
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

    // Ensure the funds are valid (ie at least 1 hash in 1hash increments)
    if funds.amount.is_zero() || funds.amount.u128() % 1000000000u128 != 0 {
        return Err(ContractError::InvalidFunds {
            message: format!(
                "sell amount must be > 0 in 1000000000nhash increments: got {}",
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

    // Just assume no rounding issues for now.
    let outstanding = funds.amount * Decimal::from_ratio(price.u128(), 1000000000u128);

    // Persist sell order
    sell_book.save(
        &order_key,
        &SellOrder {
            id: id.clone(),
            price,
            ts: env.block.time,
            seller: info.sender,
            funds: funds.amount,
            outstanding,
            denom: state.sell_denom,
        },
    )?;

    // Create response and add ID to outgoing SC `wasm` event
    let mut res = Response::new();
    res.add_attribute("action", "orderbook.sell");
    res.add_attribute("id", id);
    Ok(res)
}

// Execute a single step of the match algorithm: process a single sell.
fn try_step(deps: DepsMut) -> Result<Response, ContractError> {
    let mut res = Response::new();
    let sells = get_sell_orders(deps.as_ref())?;
    if !sells.is_empty() {
        match_sell(deps, &mut res, sells[0].clone())?;
    }
    Ok(res)
}

// Look for and match with buy orders for a given sell order.
fn match_sell(deps: DepsMut, res: &mut Response, sell: SellOrder) -> Result<(), ContractError> {
    // Create an updatable sell order
    let mut sell = sell;

    // Look for buy orders with a price >= sell price.
    let buys: Vec<BuyOrder> = get_buy_orders(deps.as_ref())?
        .into_iter()
        .filter(|buy| buy.price >= sell.price)
        .collect();

    // Match sell with any/all buy orders
    for buy in buys {
        // Execute match
        let match_res = match_orders(buy, sell.clone())?;

        // Add sends to outgoing response
        for msg in match_res.msgs {
            res.add_message(msg);
        }

        // Add a match event attribute to outgoing response
        res.add_attribute(
            "match",
            format!("buy:{},sell:{}", match_res.buy.id, match_res.sell.id),
        );

        // Update sell for the next iteration
        sell = match_res.sell.clone();

        // Persist sell order state
        update_sell_order(deps.storage, match_res.sell)?;

        // Persist buy order state
        update_buy_order(deps.storage, match_res.buy)?;

        // Nothing more we can do for this sell
        if sell.is_closed() {
            break;
        }
    }
    Ok(())
}

// The return type for matching orders
struct MatchResult {
    pub buy: BuyOrder,
    pub sell: SellOrder,
    pub msgs: Vec<CosmosMsg>,
}

// Match a buy order with a sell order.
fn match_orders(buy: BuyOrder, sell: SellOrder) -> Result<MatchResult, ContractError> {
    // Validate orders are still open
    if buy.is_closed() {
        return Err(ContractError::BuyClosed {});
    }
    if sell.is_closed() {
        return Err(ContractError::SellClosed {});
    }

    // Make sell and buy updatable
    let mut sell = sell;
    let mut buy = buy;

    // Tracks bank sends required for matching
    let mut msgs: Vec<CosmosMsg> = Vec::new();

    // Process stablecoin transfer to seller
    match sell.outstanding.cmp(&buy.funds) {
        Ordering::Less => {
            // Transfer sell.outstanding funds to seller
            let amt = coin(sell.outstanding.u128(), buy.denom.clone());
            msgs.push(
                BankMsg::Send {
                    amount: vec![amt],
                    to_address: sell.seller.clone(),
                }
                .into(),
            );
            // Reduce buy.funds by sell.outstanding
            buy.funds = (buy.funds - sell.outstanding)?;
            // Set sell.outstanding to zero
            sell.outstanding = Uint128::zero();
        }
        _ => {
            // Transfer buy.funds to seller
            let amt = coin(buy.funds.u128(), buy.denom.clone());
            msgs.push(
                BankMsg::Send {
                    amount: vec![amt],
                    to_address: sell.seller.clone(),
                }
                .into(),
            );
            // Reduce sell.outstanding by buy.funds
            sell.outstanding = (sell.outstanding - buy.funds)?;
            // Set buy.funds to zero
            buy.funds = Uint128::zero();
        }
    }

    // Process nhash transfer to buyer
    match buy.outstanding.cmp(&sell.funds) {
        Ordering::Less => {
            // Transfer buy.outstanding funds to buyer
            let amt = coin(buy.outstanding.u128(), sell.denom.clone());
            msgs.push(
                BankMsg::Send {
                    amount: vec![amt],
                    to_address: buy.buyer.clone(),
                }
                .into(),
            );
            // Reduce sell.funds by buy.outstanding
            sell.funds = (sell.funds - buy.outstanding)?;
            // Set buy.outstanding to zero
            buy.outstanding = Uint128::zero();
        }
        _ => {
            // Transfer sell.funds to buyer
            let amt = coin(sell.funds.u128(), sell.denom.clone());
            msgs.push(
                BankMsg::Send {
                    amount: vec![amt],
                    to_address: buy.buyer.clone(),
                }
                .into(),
            );
            // Reduce buy.outstanding by sell.funds
            buy.outstanding = (buy.outstanding - sell.funds)?;
            // Set sell.funds to zero
            sell.funds = Uint128::zero();
        }
    }

    // If the sell ask amount was met but not all funds were required, refund them.
    if sell.outstanding.is_zero() && !sell.funds.is_zero() {
        let amt = coin(sell.funds.u128(), sell.denom.clone());
        msgs.push(
            BankMsg::Send {
                amount: vec![amt],
                to_address: sell.seller.clone(),
            }
            .into(),
        );
        sell.funds = Uint128::zero();
    }

    Ok(MatchResult { buy, sell, msgs })
}

// Update a sell order in order book storage.
fn update_sell_order(storage: &mut dyn Storage, order: SellOrder) -> Result<(), ContractError> {
    // Ensure an order with the given ID doesn't already exist.
    let key = order.id.as_bytes();
    let mut book = sell_orders(storage);
    // Persist sell order
    if order.is_closed() {
        book.remove(&key);
    } else {
        book.save(&key, &order)?;
    }
    Ok(())
}

// Update a buy order in order book storage.
fn update_buy_order(storage: &mut dyn Storage, order: BuyOrder) -> Result<(), ContractError> {
    // Ensure an order with the given ID doesn't already exist.
    let key = order.id.as_bytes();
    let mut book = buy_orders(storage);
    // Persist sell order
    if order.is_closed() {
        book.remove(&key);
    } else {
        book.save(&key, &order)?;
    }
    Ok(())
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
        //todo!()
    }
}
