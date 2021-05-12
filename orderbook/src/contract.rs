use cosmwasm_std::{
    coin, to_binary, BankMsg, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Order,
    QueryResponse, Response, StdResult, Storage, Uint128,
};

use crate::error::ContractError;
use crate::msg::{AskOrders, BidOrders, ExecuteMsg, InitMsg, Orderbook, QueryMsg};
use crate::state::{
    ask_orders, ask_orders_read, bid_orders, bid_orders_read, config, config_read, AskOrder,
    BidOrder, State,
};

use std::cmp::Ordering;

/// Initialize and save config state.
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<Response, ContractError> {
    // Create and store config state.
    let state = State {
        ask_denom: "nhash".into(),             // nano-hash
        ask_increment: Uint128(1_000_000_000), // 1 hash
        bid_denom: msg.bid_denom,
        contract_admin: info.sender,
    };
    config(deps.storage).save(&state)?;
    Ok(Response::default())
}

/// Persist a bid or ask offer, or execute the matching algorithm.
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Bid { id, price } => try_bid(deps, env, info, id, price),
        ExecuteMsg::Ask { id, price } => try_ask(deps, env, info, id, price),
        ExecuteMsg::Match {} => try_match(deps, info, env),
    }
}

// Validate then persist a bid order for later matching.
fn try_bid(
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
            message: "invalid number of bid funds provided".into(),
        });
    }
    let funds = info.funds[0].clone();

    // Load config state
    let state = config_read(deps.storage).load()?;

    // Ensure the funds are valid
    if funds.amount.is_zero() {
        return Err(ContractError::InvalidFunds {
            message: "bid amount must be > 0".into(),
        });
    }
    if funds.denom != state.bid_denom {
        return Err(ContractError::InvalidFunds {
            message: format!(
                "invalid bid denom: got {}, require {}",
                funds.denom, state.bid_denom
            ),
        });
    }

    // Admin is not allowed bid hash, only execute the matching algorithm.
    if info.sender == state.contract_admin {
        return Err(ContractError::Unauthorized {});
    }

    // Ensure an order with the given ID doesn't already exist.
    let order_key = id.as_bytes();
    let mut book = bid_orders(deps.storage);
    if book.may_load(&order_key)?.is_some() {
        return Err(ContractError::DuplicateBid { id: id.clone() });
    }

    // Calculate and verify buy proceeds.
    let num = funds.amount.u128() * state.ask_increment.u128();
    if num % price.u128() != 0 {
        return Err(ContractError::InvalidFunds {
            message: "bid price must yield an integral for proceeds".into(),
        });
    }
    let proceeds = Uint128(num / price.u128());
    if proceeds.u128() % state.ask_increment.u128() != 0 {
        deps.api.debug(&format!("proceeds={:?}", proceeds));
        return Err(ContractError::InvalidFunds {
            message: "funds must yield a bid amount in the required increments".into(),
        });
    }

    // Persist bid order
    book.save(
        &order_key,
        &BidOrder {
            id: id.clone(),
            price,
            ts: env.block.time.nanos() / 1_000_000_000, // use seconds
            bidder: info.sender,
            funds: funds.amount,
            funds_denom: funds.denom,
            proceeds,
        },
    )?;

    // Create response and add ID to outgoing SC `wasm` event
    let mut res = Response::new();
    res.add_attribute("action", "orderbook.bid");
    res.add_attribute("id", id);
    Ok(res)
}

// Validate then persist a ask order for later matching.
fn try_ask(
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
            message: "invalid number of ask funds provided".into(),
        });
    }
    let funds = info.funds[0].clone();

    // Load config state
    let state = config_read(deps.storage).load()?;

    // Ensure the funds are valid (ie at least 1 hash in 1hash increments)
    if funds.amount.is_zero() || funds.amount.u128() % state.ask_increment.u128() != 0 {
        return Err(ContractError::InvalidFunds {
            message: format!(
                "ask amount must be > 0 in the required increments: got {}",
                funds.amount
            ),
        });
    }
    if funds.denom != state.ask_denom {
        return Err(ContractError::InvalidFunds {
            message: format!(
                "invalid ask denom: got {}, require {}",
                funds.denom, state.bid_denom
            ),
        });
    }

    // Admin is not allowed sell hash, only execute the matching algorithm.
    if info.sender == state.contract_admin {
        return Err(ContractError::Unauthorized {});
    }

    // Ensure an order with the given ID doesn't already exist.
    let order_key = id.as_bytes();
    let mut book = ask_orders(deps.storage);
    if book.may_load(&order_key)?.is_some() {
        return Err(ContractError::DuplicateAsk { id: id.clone() });
    }

    // Calculate sell proceeds
    let proceeds = funds.amount * Decimal::from_ratio(price, state.ask_increment);

    // Persist ask order
    book.save(
        &order_key,
        &AskOrder {
            id: id.clone(),
            price,
            ts: env.block.time.nanos() / 1_000_000_000, // use seconds
            asker: info.sender,
            funds: funds.amount,
            funds_denom: funds.denom,
            proceeds,
        },
    )?;

    // Create response and add ID to outgoing SC `wasm` event
    let mut res = Response::new();
    res.add_attribute("action", "orderbook.ask");
    res.add_attribute("id", id);
    Ok(res)
}

// Execute the match algorithm.
fn try_match(deps: DepsMut, info: MessageInfo, env: Env) -> Result<Response, ContractError> {
    // Load config state
    let state = config_read(deps.storage).load()?;

    // Only the admin can execute matching.
    if info.sender != state.contract_admin {
        return Err(ContractError::Unauthorized {});
    }

    // Create aggregate response and get the BFT time of the current block.
    let mut res = Response::new();
    let ts = env.block.time.nanos() / 1_000_000_000; // use seconds

    // Query and filter ask orders
    let asks: Vec<AskOrder> = get_ask_orders(deps.as_ref())?
        .into_iter()
        .filter(|ask| ask.ts < ts) // Ignore asks in the current block
        .collect();

    // Match each ask in price/time order
    for ask in asks {
        // Create an updatable ask order
        let mut ask = ask;

        // Look for bid orders with a price >= ask price, ignoring bids in the current block.
        let bids: Vec<BidOrder> = get_bid_orders(deps.as_ref())?
            .into_iter()
            .filter(|bid| bid.price >= ask.price && bid.ts < ts)
            .collect();

        // Match ask with any/all bid orders
        for bid in bids {
            // Execute match
            let match_res = match_orders(bid, ask.clone())?;

            // Add bank sends to outgoing response
            for msg in match_res.msgs {
                res.add_message(msg);
            }

            // Add a match event attribute to outgoing response
            res.add_attribute(
                "orderbook.match",
                format!("bid:{},ask:{}", match_res.bid.id, match_res.ask.id),
            );

            // Update ask for the next iteration
            ask = match_res.ask.clone();

            // Persist order state
            update_ask_order(deps.storage, match_res.ask)?;
            update_bid_order(deps.storage, match_res.bid)?;

            // Stop if ask is closed
            if ask.is_closed() {
                break;
            }
        }
    }

    // Done
    Ok(res)
}

// The return type for matching orders
struct MatchResult {
    pub bid: BidOrder,
    pub ask: AskOrder,
    pub msgs: Vec<CosmosMsg>,
}

// Match a bid order with a ask order.
fn match_orders(bid: BidOrder, ask: AskOrder) -> Result<MatchResult, ContractError> {
    // Validate orders are still open
    if bid.is_closed() {
        return Err(ContractError::BidClosed {});
    }
    if ask.is_closed() {
        return Err(ContractError::AskClosed {});
    }

    // Make ask and bid updatable
    let mut ask = ask;
    let mut bid = bid;

    // Tracks bank sends required for matching
    let mut msgs: Vec<CosmosMsg> = Vec::new();

    // Process stablecoin transfer to asker
    match ask.proceeds.cmp(&bid.funds) {
        Ordering::Less => {
            // Transfer ask.proceeds funds to asker
            let amt = coin(ask.proceeds.u128(), bid.funds_denom.clone());
            msgs.push(
                BankMsg::Send {
                    amount: vec![amt],
                    to_address: ask.asker.to_string(),
                }
                .into(),
            );
            // Reduce bid.funds by ask.proceeds
            bid.funds = Uint128(bid.funds.u128() - ask.proceeds.u128());
            // Set ask.proceeds to zero
            ask.proceeds = Uint128::zero();
        }
        _ => {
            // Transfer bid.funds to asker
            let amt = coin(bid.funds.u128(), bid.funds_denom.clone());
            msgs.push(
                BankMsg::Send {
                    amount: vec![amt],
                    to_address: ask.asker.to_string(),
                }
                .into(),
            );
            // Reduce ask.proceeds by bid.funds
            ask.proceeds = Uint128(ask.proceeds.u128() - bid.funds.u128());
            // Set bid.funds to zero
            bid.funds = Uint128::zero();
        }
    }

    // Process nhash transfer to bidder
    match bid.proceeds.cmp(&ask.funds) {
        Ordering::Less => {
            // Transfer bid.proceeds funds to bidder
            let amt = coin(bid.proceeds.u128(), ask.funds_denom.clone());
            msgs.push(
                BankMsg::Send {
                    amount: vec![amt],
                    to_address: bid.bidder.to_string(),
                }
                .into(),
            );
            // Reduce ask.funds by bid.proceeds
            ask.funds = Uint128(ask.funds.u128() - bid.proceeds.u128());
            // Set bid.proceeds to zero
            bid.proceeds = Uint128::zero();
        }
        _ => {
            // Transfer ask.funds to bidder
            let amt = coin(ask.funds.u128(), ask.funds_denom.clone());
            msgs.push(
                BankMsg::Send {
                    amount: vec![amt],
                    to_address: bid.bidder.to_string(),
                }
                .into(),
            );
            // Reduce bid.proceeds by ask.funds
            bid.proceeds = Uint128(bid.proceeds.u128() - ask.funds.u128());
            // Set ask.funds to zero
            ask.funds = Uint128::zero();
        }
    }

    // If the ask amount was met but not all funds were required, refund them.
    if ask.proceeds.is_zero() && !ask.funds.is_zero() {
        let refund = coin(ask.funds.u128(), ask.funds_denom.clone());
        msgs.push(
            BankMsg::Send {
                amount: vec![refund],
                to_address: ask.asker.to_string(),
            }
            .into(),
        );
        ask.funds = Uint128::zero();
    }

    Ok(MatchResult { bid, ask, msgs })
}

// Update an ask in orderbook storage.
fn update_ask_order(storage: &mut dyn Storage, order: AskOrder) -> Result<(), ContractError> {
    // Ensure an order with the given ID doesn't already exist.
    let key = order.id.as_bytes();
    let mut book = ask_orders(storage);
    // Persist ask order
    if order.is_closed() {
        book.remove(&key);
    } else {
        book.save(&key, &order)?;
    }
    Ok(())
}

// Update a bid in orderbook storage.
fn update_bid_order(storage: &mut dyn Storage, order: BidOrder) -> Result<(), ContractError> {
    // Ensure an order with the given ID doesn't already exist.
    let key = order.id.as_bytes();
    let mut book = bid_orders(storage);
    // Persist bid order
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
        QueryMsg::GetBidOrders {} => try_get_bid_orders(deps),
        QueryMsg::GetAskOrders {} => try_get_ask_orders(deps),
        QueryMsg::GetOrderbook {} => try_get_orderbook(deps),
    }
}

// Read all bid orders into memory, sort by price/ts, then serialize to JSON.
fn try_get_bid_orders(deps: Deps) -> Result<QueryResponse, ContractError> {
    // Query sorted bid orders, checking for errors
    let bid_orders = get_bid_orders(deps)?;
    // Serialize and return
    let bin = to_binary(&BidOrders { bid_orders })?;
    Ok(bin)
}

// Read all bid orders into memory then sort by price, timestamp.
fn get_bid_orders(deps: Deps) -> Result<Vec<BidOrder>, ContractError> {
    // Read all bid orders
    let bid_orders: StdResult<Vec<_>> = bid_orders_read(deps.storage)
        .range(None, None, Order::Ascending)
        .map(|item| {
            let (_, bid_order) = item?;
            Ok(bid_order)
        })
        .collect();

    // Check for error
    let mut bid_orders = bid_orders?;

    // Sort by price, then time.
    bid_orders.sort_by(|a, b| {
        if a.price != b.price {
            b.price.cmp(&a.price) // flip comparison for best price first
        } else {
            a.ts.cmp(&b.ts)
        }
    });

    // Return sorted in price-time order
    Ok(bid_orders)
}

// Read all ask orders into memory, sort by amount/ts, then serialize to JSON.
fn try_get_ask_orders(deps: Deps) -> Result<QueryResponse, ContractError> {
    // Query sorted ask orders, checking for errors
    let ask_orders = get_ask_orders(deps)?;
    // Serialize and return
    let bin = to_binary(&AskOrders { ask_orders })?;
    Ok(bin)
}

// Read all ask orders into memory then sort by price, timestamp.
fn get_ask_orders(deps: Deps) -> Result<Vec<AskOrder>, ContractError> {
    // Read all ask orders
    let ask_orders: StdResult<Vec<_>> = ask_orders_read(deps.storage)
        .range(None, None, Order::Ascending)
        .map(|item| {
            let (_, ask_order) = item?;
            Ok(ask_order)
        })
        .collect();

    // Check for error
    let mut ask_orders = ask_orders?;

    // Sort by price, then time.
    ask_orders.sort_by(|a, b| {
        if a.price != b.price {
            b.price.cmp(&a.price) // flip comparison for best price first
        } else {
            a.ts.cmp(&b.ts)
        }
    });

    // Return sorted in price-time order
    Ok(ask_orders)
}

// Read all ask orders into memory, sort by price/ts, then serialize to JSON.
fn try_get_orderbook(deps: Deps) -> Result<QueryResponse, ContractError> {
    // Query sorted bid orders, checking for errors
    let bid_orders = get_bid_orders(deps)?;
    // Query sorted ask orders, checking for errors
    let ask_orders = get_ask_orders(deps)?;
    // Serialize and return
    let bin = to_binary(&Orderbook {
        bid_orders,
        ask_orders,
    })?;
    Ok(bin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{from_binary, Addr, Api};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn valid_init() {
        // Create mock deps.
        let mut deps = mock_dependencies(&[]);

        // Init
        let res = instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                bid_denom: "stablecoin".into(),
            },
        )
        .unwrap();

        // Ensure no messages were created.
        assert_eq!(0, res.messages.len());

        // Read state
        let config_state = config_read(&deps.storage).load().unwrap();

        // Ensure expected state values
        assert_eq!(config_state.ask_denom, "nhash");
        assert_eq!(config_state.ask_increment, Uint128(1_000_000_000));
        assert_eq!(config_state.bid_denom, "stablecoin");
    }

    #[test]
    fn persist_bid_order() {
        // Create mock deps.
        let mut deps = mock_dependencies(&[]);

        // Init
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                bid_denom: "stablecoin".into(),
            },
        )
        .unwrap();

        // Buy 10 hash at 1 stablecoin/hash price
        let funds = coin(10, "stablecoin");
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &[funds]),
            ExecuteMsg::Bid {
                id: "test-bid".into(),
                price: Uint128(1),
            },
        )
        .unwrap();

        // Query bids from orderbook
        let bin = query(deps.as_ref(), mock_env(), QueryMsg::GetBidOrders {}).unwrap();

        // Ensure bid side of orderbook has the expected state
        let rep: BidOrders = from_binary(&bin).unwrap();
        deps.api.debug(&format!("{:?}", rep));
        assert_eq!(rep.bid_orders.len(), 1);
        assert_eq!(rep.bid_orders[0].id, "test-bid");
        assert_eq!(rep.bid_orders[0].price, Uint128(1));
        assert_eq!(rep.bid_orders[0].funds, Uint128(10));
        assert_eq!(rep.bid_orders[0].proceeds, Uint128(10_000_000_000));
    }

    #[test]
    fn persist_ask_order() {
        // Create mock deps.
        let mut deps = mock_dependencies(&[]);

        // Init
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                bid_denom: "stablecoin".into(),
            },
        )
        .unwrap();

        // Sell 10 hash at 1 stablecoin/hash price
        let funds = coin(10_000_000_000, "nhash");
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[funds]),
            ExecuteMsg::Ask {
                id: "test-ask".into(),
                price: Uint128(1),
            },
        )
        .unwrap();

        // Query asks from orderbook
        let bin = query(deps.as_ref(), mock_env(), QueryMsg::GetAskOrders {}).unwrap();

        // Ensure bid side of orderbook has the expected state
        let rep: AskOrders = from_binary(&bin).unwrap();
        deps.api.debug(&format!("{:?}", rep));
        assert_eq!(rep.ask_orders.len(), 1);
        assert_eq!(rep.ask_orders[0].id, "test-ask");
        assert_eq!(rep.ask_orders[0].price, Uint128(1));
        assert_eq!(rep.ask_orders[0].funds, Uint128(10_000_000_000));
        assert_eq!(rep.ask_orders[0].proceeds, Uint128(10));
    }

    #[test]
    fn direct_match() {
        // Create mock deps.
        let mut deps = mock_dependencies(&[]);

        // Init
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                bid_denom: "stablecoin".into(),
            },
        )
        .unwrap();

        // Buy 10 hash at 1 stablecoin/hash price
        let funds = coin(10, "stablecoin");
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &[funds]),
            ExecuteMsg::Bid {
                id: "test-bid".into(),
                price: Uint128(1),
            },
        )
        .unwrap();

        // Sell 10 hash at 1 stablecoin/hash price
        let funds = coin(10_000_000_000, "nhash");
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[funds]),
            ExecuteMsg::Ask {
                id: "test-ask".into(),
                price: Uint128(1),
            },
        )
        .unwrap();

        // Query the orderbook
        let bin = query(deps.as_ref(), mock_env(), QueryMsg::GetOrderbook {}).unwrap();

        // Ensure both orders were added to the orderbook.
        let rep: Orderbook = from_binary(&bin).unwrap();
        assert_eq!(rep.bid_orders.len(), 1);
        assert_eq!(rep.ask_orders.len(), 1);

        deps.api.debug(&format!("{:?}", rep));

        // Move block time forward so it seems like we're matching in the next block.
        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(3);

        // Execute a match
        let res = execute(
            deps.as_mut(),
            env,
            mock_info("admin", &[]), // Admin must execute match
            ExecuteMsg::Match {},
        )
        .unwrap();

        // Ensure we got two bank sends
        assert_eq!(res.messages.len(), 2);

        // Ensure we got the expected bank transfer amounts.
        res.messages.into_iter().for_each(|msg| match msg {
            CosmosMsg::Bank(BankMsg::Send {
                amount, to_address, ..
            }) => {
                assert_eq!(amount.len(), 1);
                if to_address == Addr::unchecked("asker") {
                    let expected_asker_amount = coin(10, "stablecoin");
                    assert_eq!(amount[0], expected_asker_amount);
                } else {
                    assert_eq!(to_address, "bidder");
                    let expected_bidder_amount = coin(10_000_000_000, "nhash");
                    assert_eq!(amount[0], expected_bidder_amount);
                }
            }
            _ => panic!("unexpected message type"),
        });

        // Ensure we got one match event attribute
        assert_eq!(res.attributes.len(), 1);
        assert_eq!(res.attributes[0].key, "orderbook.match");
        assert_eq!(res.attributes[0].value, "bid:test-bid,ask:test-ask");

        // Ensure both orders were removed from the orderbook.
        let bin = query(deps.as_ref(), mock_env(), QueryMsg::GetOrderbook {}).unwrap();
        let rep: Orderbook = from_binary(&bin).unwrap();
        assert_eq!(rep.bid_orders.len(), 0);
        assert_eq!(rep.ask_orders.len(), 0);
    }

    #[test]
    fn partial_match_bid() {
        // Create mock deps.
        let mut deps = mock_dependencies(&[]);

        // Init
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                bid_denom: "stablecoin".into(),
            },
        )
        .unwrap();

        // Buy 10 hash at 1 stablecoin/hash price
        let funds = coin(10, "stablecoin");
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &[funds]),
            ExecuteMsg::Bid {
                id: "test-bid".into(),
                price: Uint128(1),
            },
        )
        .unwrap();

        // Sell 5 hash at 1 stablecoin/hash price
        let funds = coin(5_000_000_000, "nhash");
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[funds]),
            ExecuteMsg::Ask {
                id: "test-ask".into(),
                price: Uint128(1),
            },
        )
        .unwrap();

        // Query the orderbook
        let bin = query(deps.as_ref(), mock_env(), QueryMsg::GetOrderbook {}).unwrap();

        // Ensure both orders were added to the orderbook.
        let rep: Orderbook = from_binary(&bin).unwrap();
        assert_eq!(rep.bid_orders.len(), 1);
        assert_eq!(rep.ask_orders.len(), 1);

        deps.api.debug(&format!("{:?}", rep));

        // Move block time forward so it seems like we're matching in the next block.
        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(3);

        // Execute a match
        let res = execute(
            deps.as_mut(),
            env,
            mock_info("admin", &[]), // Admin must execute match
            ExecuteMsg::Match {},
        )
        .unwrap();

        // Ensure we got two bank sends
        assert_eq!(res.messages.len(), 2);

        // Ensure we got the expected bank transfer amounts.
        res.messages.into_iter().for_each(|msg| match msg {
            CosmosMsg::Bank(BankMsg::Send {
                amount, to_address, ..
            }) => {
                assert_eq!(amount.len(), 1);
                if to_address == Addr::unchecked("asker") {
                    let expected_amount = coin(5, "stablecoin");
                    assert_eq!(amount[0], expected_amount);
                } else {
                    assert_eq!(to_address, "bidder");
                    let expected_amount = coin(5_000_000_000, "nhash");
                    assert_eq!(amount[0], expected_amount);
                }
            }
            _ => panic!("unexpected message type"),
        });

        // Ensure we got one match event attribute
        assert_eq!(res.attributes.len(), 1);
        assert_eq!(res.attributes[0].key, "orderbook.match");
        assert_eq!(res.attributes[0].value, "bid:test-bid,ask:test-ask");

        // Ensure the bid order was updated in the orderbook.
        let bin = query(deps.as_ref(), mock_env(), QueryMsg::GetOrderbook {}).unwrap();
        let rep: Orderbook = from_binary(&bin).unwrap();
        assert_eq!(rep.bid_orders.len(), 1);
        assert_eq!(rep.ask_orders.len(), 0);

        // Verfiy there are still 5 hash proceeds in the bid order
        assert_eq!(rep.bid_orders[0].id, "test-bid");
        assert_eq!(rep.bid_orders[0].price, Uint128(1));
        assert_eq!(rep.bid_orders[0].funds, Uint128(5));
        assert_eq!(rep.bid_orders[0].proceeds, Uint128(5_000_000_000));
    }

    #[test]
    fn partial_match_ask() {
        // Create mock deps.
        let mut deps = mock_dependencies(&[]);

        // Init
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                bid_denom: "stablecoin".into(),
            },
        )
        .unwrap();

        // Buy 5 hash at 1 stablecoin/hash price
        let funds = coin(5, "stablecoin");
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &[funds]),
            ExecuteMsg::Bid {
                id: "test-bid".into(),
                price: Uint128(1),
            },
        )
        .unwrap();

        // Sell 10 hash at 1 stablecoin/hash price
        let funds = coin(10_000_000_000, "nhash");
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[funds]),
            ExecuteMsg::Ask {
                id: "test-ask".into(),
                price: Uint128(1),
            },
        )
        .unwrap();

        // Query the orderbook
        let bin = query(deps.as_ref(), mock_env(), QueryMsg::GetOrderbook {}).unwrap();

        // Ensure both orders were added to the orderbook.
        let rep: Orderbook = from_binary(&bin).unwrap();
        assert_eq!(rep.bid_orders.len(), 1);
        assert_eq!(rep.ask_orders.len(), 1);

        deps.api.debug(&format!("{:?}", rep));

        // Move block time forward so it seems like we're matching in the next block.
        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(3);

        // Execute a match
        let res = execute(
            deps.as_mut(),
            env,
            mock_info("admin", &[]), // Admin must execute match
            ExecuteMsg::Match {},
        )
        .unwrap();

        // Ensure we got two bank sends
        assert_eq!(res.messages.len(), 2);

        // Ensure we got the expected bank transfer amounts.
        res.messages.into_iter().for_each(|msg| match msg {
            CosmosMsg::Bank(BankMsg::Send {
                amount, to_address, ..
            }) => {
                assert_eq!(amount.len(), 1);
                if to_address == Addr::unchecked("asker") {
                    let expected_amount = coin(5, "stablecoin");
                    assert_eq!(amount[0], expected_amount);
                } else {
                    assert_eq!(to_address, "bidder");
                    let expected_amount = coin(5_000_000_000, "nhash");
                    assert_eq!(amount[0], expected_amount);
                }
            }
            _ => panic!("unexpected message type"),
        });

        // Ensure we got one match event attribute
        assert_eq!(res.attributes.len(), 1);
        assert_eq!(res.attributes[0].key, "orderbook.match");
        assert_eq!(res.attributes[0].value, "bid:test-bid,ask:test-ask");

        // Ensure the ask order was updated in the orderbook.
        let bin = query(deps.as_ref(), mock_env(), QueryMsg::GetOrderbook {}).unwrap();
        let rep: Orderbook = from_binary(&bin).unwrap();
        assert_eq!(rep.bid_orders.len(), 0);
        assert_eq!(rep.ask_orders.len(), 1);

        // Verify there are still 5 stablecoin proceeds in the ask order
        assert_eq!(rep.ask_orders[0].id, "test-ask");
        assert_eq!(rep.ask_orders[0].price, Uint128(1));
        assert_eq!(rep.ask_orders[0].funds, Uint128(5_000_000_000));
        assert_eq!(rep.ask_orders[0].proceeds, Uint128(5));
    }

    #[test]
    fn unauthorized_bid() {
        // Create mock deps.
        let mut deps = mock_dependencies(&[]);

        // Init
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                bid_denom: "stablecoin".into(),
            },
        )
        .unwrap();

        // Buy 5 hash at 1 stablecoin/hash price
        let funds = coin(5, "stablecoin");
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[funds]), // Admin cannot place bid orders
            ExecuteMsg::Bid {
                id: "test-bid".into(),
                price: Uint128(1),
            },
        )
        .unwrap_err();

        // Ensure we go the expected error
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!("unexpected error type"),
        }
    }

    #[test]
    fn unauthorized_ask() {
        // Create mock deps.
        let mut deps = mock_dependencies(&[]);

        // Init
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                bid_denom: "stablecoin".into(),
            },
        )
        .unwrap();

        // Sell 10 hash at 1 stablecoin/hash price
        let funds = coin(10_000_000_000, "nhash");
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[funds]), // Admin cannot place ask orders
            ExecuteMsg::Ask {
                id: "test-ask".into(),
                price: Uint128(1),
            },
        )
        .unwrap_err();

        // Ensure we go the expected error
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!("unexpected error type"),
        }
    }

    #[test]
    fn unauthorized_match() {
        // Create mock deps.
        let mut deps = mock_dependencies(&[]);

        // Init
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                bid_denom: "stablecoin".into(),
            },
        )
        .unwrap();

        // Execute a match
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[]), // Admin must execute match
            ExecuteMsg::Match {},
        )
        .unwrap_err();

        // Ensure we go the expected error
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!("unexpected error type"),
        }
    }

    #[test]
    fn invalid_bid_amount() {
        // Create mock deps.
        let mut deps = mock_dependencies(&[]);

        // Init
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                bid_denom: "stablecoin".into(),
            },
        )
        .unwrap();

        // Attempt to buy 1 hash at 15 stablecoin/hash price yielding fractional nhash proceeds
        let funds = coin(1, "stablecoin");
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &[funds]),
            ExecuteMsg::Bid {
                id: "test-bid".into(),
                price: Uint128(15),
            },
        )
        .unwrap_err();

        // Ensure we go the expected error
        match err {
            ContractError::InvalidFunds { message } => {
                assert_eq!(message, "bid price must yield an integral for proceeds")
            }
            _ => panic!("unexpected error type"),
        }
    }

    #[test]
    fn invalid_bid_amount_increment() {
        // Create mock deps.
        let mut deps = mock_dependencies(&[]);

        // Init
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                bid_denom: "stablecoin".into(),
            },
        )
        .unwrap();

        // Attempt to buy < 1hash at 15 stablecoin/hash price
        let funds = coin(3, "stablecoin");
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("bidder", &[funds]),
            ExecuteMsg::Bid {
                id: "test-bid".into(),
                price: Uint128(15),
            },
        )
        .unwrap_err();

        // Ensure we go the expected error
        match err {
            ContractError::InvalidFunds { message } => {
                assert_eq!(
                    message,
                    "funds must yield a bid amount in the required increments"
                )
            }
            _ => panic!("unexpected error type"),
        }
    }

    #[test]
    fn invalid_ask_amount() {
        // Create mock deps.
        let mut deps = mock_dependencies(&[]);

        // Init
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                bid_denom: "stablecoin".into(),
            },
        )
        .unwrap();

        // Attempt to sell < 1 hash at 1 stablecoin/hash price
        let funds = coin(123456789, "nhash");
        let err = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("asker", &[funds]),
            ExecuteMsg::Ask {
                id: "test-ask".into(),
                price: Uint128(1),
            },
        )
        .unwrap_err();

        // Ensure we go the expected error
        match err {
            ContractError::InvalidFunds { message } => assert_eq!(
                message,
                "ask amount must be > 0 in the required increments: got 123456789"
            ),
            _ => panic!("unexpected error type"),
        }
    }
}
