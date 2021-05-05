use cosmwasm_std::{
    coin, has_coins, to_binary, Addr, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    QueryResponse, Response, Uint128,
};

use provwasm_std::{withdraw_coins, ProvenanceMsg, ProvenanceQuerier};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg, TraderStateResponse};
use crate::state::{config, config_read, trader_bucket, trader_bucket_read, State, TraderState};

/// Initialize the smart contract config state.
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    config(deps.storage).save(&State {
        contract_admin: info.sender,
        security: msg.security,
        stablecoin: msg.stablecoin,
    })?;
    Ok(Response::default())
}

/// Handle messages that will add traders and allow them to buy/sell a security.
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    match msg {
        ExecuteMsg::AddTrader { address } => try_add_trader(deps, info, address),
        ExecuteMsg::BuyStock { amount } => try_buy_stock(deps, env, info, amount),
        ExecuteMsg::SellStock { amount } => try_sell_stock(deps, info, amount),
    }
}

// Query for account stablecoin balance and create trader config, setting loan cap to 9x balance.
fn try_add_trader(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    // Load contract state and validate the message sender is the contact admin.
    let state = config_read(deps.storage).load()?;
    if info.sender != state.contract_admin {
        return Err(ContractError::Unauthorized {});
    }

    // Query trader's stablecoin balance, ensuring it is non-zero.
    // let balance: Coin = deps.querier.query_balance(&address, &state.stablecoin)?;
    // if balance.amount.is_zero() {
    //     return Err(ContractError::InsufficientFunds {});
    // }

    // Load trader config bucket
    let mut bucket = trader_bucket(deps.storage);

    // Initialize and save trader config state if necessary.
    let trader_key = deps.api.addr_canonicalize(&address)?;
    if bucket.may_load(&trader_key)?.is_none() {
        bucket.save(
            &trader_key,
            &TraderState {
                loan_cap: Uint128(10_000_000_000_u128),
                loans: Uint128::zero(),
            },
        )?;
    }

    // Add grant to response.
    Ok(Response::default())
}

// Allow a trader to buy stock, with borrowing up to a pre-configured loan cap.
fn try_buy_stock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    // Error if buy amount is zero or too many funds sent
    if amount.is_zero() || info.funds.len() > 1 {
        return Err(ContractError::InvalidBuy {});
    }

    // Error if trader sent zero funds and has reached or exceeded the loan cap
    let trader_key = deps.api.addr_canonicalize(&info.sender.to_string())?;
    let trader_state = trader_bucket_read(deps.storage).load(&trader_key)?;

    if info.funds.is_empty() && trader_state.loans >= trader_state.loan_cap {
        return Err(ContractError::LoanCapExceeded {
            amount,
            loans: trader_state.loans,
            loan_cap: trader_state.loan_cap,
        });
    }

    // Load security and stablecoin marker denoms.
    let config_state = config_read(deps.storage).load()?;
    let security: &str = &config_state.security;
    let stablecoin: &str = &config_state.stablecoin;

    // Validate that any funds sent have the correct denom
    if info.funds.len() == 1 && info.funds[0].denom != stablecoin {
        return Err(ContractError::InvalidFundsDenom {});
    }

    // Determine cost of purchase
    let price: Coin = stock_price(deps.as_ref(), amount.u128(), security, stablecoin);

    // Create response type we can update on the fly
    let mut res = Response::new();

    // Trader didn't sent enough to cover the purchase. Determine loan amount and ensure loan cap
    // isn't exceeded.
    if !has_coins(info.funds.as_slice(), &price) {
        // Determine amount to loan
        let sent_amount = if info.funds.len() == 1 {
            info.funds[0].amount
        } else {
            Uint128::zero()
        };
        let loan_amount = price.amount.u128() - sent_amount.u128();

        // Ensure trader is under loan cap after borrowing.
        let max_loan_amount = trader_state.loan_cap.u128() - trader_state.loans.u128();
        if loan_amount > max_loan_amount {
            return Err(ContractError::LoanCapExceeded {
                amount,
                loans: trader_state.loans,
                loan_cap: trader_state.loan_cap,
            });
        }

        // Escrow loan amount from the stablecoin loan pool marker into the contract
        let loan_msg = withdraw_coins(stablecoin, loan_amount, stablecoin, env.contract.address)?;
        res.add_message(loan_msg);

        // Update the trader's loan total
        trader_bucket(deps.storage).update(&trader_key, |opt| -> Result<_, ContractError> {
            match opt {
                Some(mut ts) => {
                    ts.loans += Uint128(loan_amount);
                    Ok(ts)
                }
                None => Err(ContractError::UnknownTrader {}),
            }
        })?;

    // Issue a refund if the funds sent aren't exactly the amount necessary.
    } else if info.funds.len() == 1 && info.funds[0].amount > price.amount {
        let refund_amount = info.funds[0].amount.u128() - price.amount.u128();
        let refund = coin(refund_amount, stablecoin);
        let refund_msg: CosmosMsg<ProvenanceMsg> = CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![refund],
        });
        res.add_message(refund_msg);
    }

    // Withdraw stock to trader's account.
    let stock_msg = withdraw_coins(security, amount.u128(), security, info.sender)?;
    res.add_message(stock_msg);

    Ok(res)
}

// Determine the purchase price for a number of shares.
fn stock_price(_deps: Deps, shares: u128, _security: &str, stablecoin: &str) -> Coin {
    // TODO: Here's where we'd query an oracle smart contract for price of security in stablecoin.
    // For now, assume a one-to-one value
    let price_per_share: u128 = 1;
    coin(price_per_share * shares, stablecoin)
}

// Sell stock, paying off any loans first.
fn try_sell_stock(
    deps: DepsMut,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    // Ensure proper funds are sent for sells
    if amount.is_zero() || info.funds.len() != 1 {
        return Err(ContractError::InvalidSell {});
    }

    // Load trader state
    let trader_key = deps.api.addr_canonicalize(&info.sender.to_string())?;
    let trader_state = trader_bucket_read(deps.storage).load(&trader_key)?;

    // Load security and stablecoin marker denoms.
    let config_state = config_read(deps.storage).load()?;
    let security: &str = &config_state.security;
    let security_pool: Addr = get_marker_address(deps.as_ref(), security)?;
    let stablecoin: &str = &config_state.stablecoin;
    let stablecoin_pool: Addr = get_marker_address(deps.as_ref(), stablecoin)?;

    // Ensure the trader sent the correct amount of stock
    if info.funds[0].denom != security || amount != info.funds[0].amount {
        return Err(ContractError::InvalidSell {});
    }

    // Create response type we can update on the fly
    let mut res = Response::new();

    // If the trader has no loans, just transfer the stock to the security pool and send
    // escrowed funds to the sender.
    let proceeds = stock_price(deps.as_ref(), amount.u128(), security, stablecoin);
    if trader_state.loans.is_zero() {
        // Send stablecoin to trader
        let bank_msg: CosmosMsg<ProvenanceMsg> = CosmosMsg::Bank(BankMsg::Send {
            amount: vec![proceeds],
            to_address: info.sender.to_string(),
        });
        res.add_message(bank_msg);

    // Trader needs to pay back loans, but gets some stablecoin from the sale.
    } else if proceeds.amount > trader_state.loans {
        // Send the entire loan amount back to the loan pool
        let loan_amount = coin(trader_state.loans.u128(), stablecoin);
        let loan_msg: CosmosMsg<ProvenanceMsg> = CosmosMsg::Bank(BankMsg::Send {
            amount: vec![loan_amount],
            to_address: stablecoin_pool.to_string(),
        });
        res.add_message(loan_msg);

        // Determine the amount to send to the trader
        let net = proceeds.amount.u128() - trader_state.loans.u128();
        let net_amount = coin(net, stablecoin);
        let net_msg: CosmosMsg<ProvenanceMsg> = CosmosMsg::Bank(BankMsg::Send {
            amount: vec![net_amount],
            to_address: info.sender.to_string(),
        });
        res.add_message(net_msg);

        // Reset the trader's loan total back to zero
        trader_bucket(deps.storage).update(&trader_key, |opt| -> Result<_, ContractError> {
            match opt {
                Some(mut ts) => {
                    ts.loans = Uint128::zero();
                    Ok(ts)
                }
                None => Err(ContractError::UnknownTrader {}),
            }
        })?;

    // Just put the proceeds from the sale towards debt and transfer money back to the
    // stablecoin loan pool.
    } else {
        // Send proceeds back to the loan pool
        let bank_msg: CosmosMsg<ProvenanceMsg> = CosmosMsg::Bank(BankMsg::Send {
            amount: vec![proceeds.clone()],
            to_address: stablecoin_pool.to_string(),
        });
        res.add_message(bank_msg);

        // Mark off trader loan debt.
        let updated_loans = Uint128(trader_state.loans.u128() - proceeds.amount.u128());
        trader_bucket(deps.storage).update(&trader_key, |opt| -> Result<_, ContractError> {
            match opt {
                Some(mut ts) => {
                    ts.loans = updated_loans;
                    Ok(ts)
                }
                None => Err(ContractError::UnknownTrader {}),
            }
        })?;
    }

    // Send security back to stock pool
    let stock_msg: CosmosMsg<ProvenanceMsg> = CosmosMsg::Bank(BankMsg::Send {
        amount: info.funds,
        to_address: security_pool.to_string(),
    });
    res.add_message(stock_msg);

    Ok(res)
}

// Get the address for a marker or return an error if the marker doesn't exist.
fn get_marker_address(deps: Deps, denom: &str) -> Result<Addr, ContractError> {
    let querier = ProvenanceQuerier::new(&deps.querier);
    let marker = querier.get_marker_by_denom(denom)?;
    Ok(marker.address)
}

/// Handle query requests for trader loans
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<QueryResponse, ContractError> {
    match msg {
        QueryMsg::GetTraderState { address } => try_get_trader_state(deps, address),
    }
}

// Query for trader loan cap and debt.
fn try_get_trader_state(deps: Deps, address: String) -> Result<QueryResponse, ContractError> {
    // Load state
    let trader_key = deps.api.addr_canonicalize(&address)?;
    let trader_state = trader_bucket_read(deps.storage).load(&trader_key)?;
    let state = config_read(deps.storage).load()?;
    // Get the amount of stock for the trader.
    let security = match deps.querier.query_balance(&address, &state.security) {
        Ok(balance) => balance.amount,
        Err(_) => Uint128::zero(),
    };
    // Get the amount of stablecoin for the trader.
    let stablecoin = match deps.querier.query_balance(&address, &state.stablecoin) {
        Ok(balance) => balance.amount,
        Err(_) => Uint128::zero(),
    };
    // Serialize and return response
    let bin = to_binary(&TraderStateResponse {
        security,
        stablecoin,
        loans: trader_state.loans,
        loan_cap: trader_state.loan_cap,
    })?;
    Ok(bin)
}

/// Called when migrating a contract instance to a new code ID.
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // For now, we do nothing
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use provwasm_mocks::{mock_dependencies, must_read_binary_file};
    use provwasm_std::{Marker, MarkerMsgParams, ProvenanceMsgParams};

    // A helper function that will extract marker message params from a custom cosmos message.
    fn unwrap_marker_params(msg: &CosmosMsg<ProvenanceMsg>) -> &MarkerMsgParams {
        match &msg {
            CosmosMsg::Custom(msg) => match &msg.params {
                ProvenanceMsgParams::Marker(mp) => mp,
                _ => panic!("unexpected provenance params"),
            },
            _ => panic!("unexpected cosmos message"),
        }
    }

    #[test]
    fn valid_init() {
        // Create mocks.
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("admin", &[]);

        // Give the contract a name
        let msg = InitMsg {
            security: "security".into(),
            stablecoin: "stablecoin".into(),
        };

        // Ensure no messages were created.
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Read state
        let config_state = config_read(&deps.storage).load().unwrap();
        assert_eq!(config_state.security, "security");
        assert_eq!(config_state.stablecoin, "stablecoin");
    }

    #[test]
    fn add_trader() {
        // Create mocks.
        let mut deps = mock_dependencies(&[]);
        let stablecoins = coin(0, "stablecoin");
        deps.querier
            .base
            .update_balance("trader", vec![stablecoins]);

        // Init so we have config state.
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                security: "security".into(),
                stablecoin: "stablecoin".into(),
            },
        )
        .unwrap(); // panics on error

        // Onboard the trader (sets trader state, including loan cap).
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            ExecuteMsg::AddTrader {
                address: "trader".into(),
            },
        )
        .unwrap(); // panics on error

        // Query trader state
        let bin = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetTraderState {
                address: "trader".into(),
            },
        )
        .unwrap(); // panics on error

        // Ensure trader state query response has expected values
        let rep: TraderStateResponse = from_binary(&bin).unwrap();
        assert_eq!(
            rep,
            TraderStateResponse {
                security: Uint128::zero(),
                stablecoin: Uint128::zero(),
                loans: Uint128::zero(),
                loan_cap: Uint128(10_000_000_000_u128),
            }
        );
    }

    #[test]
    fn buy_with_funds() {
        // Create mocks.
        let mut deps = mock_dependencies(&[]);
        let stablecoins = coin(100, "stablecoin");
        deps.querier
            .base
            .update_balance("trader", vec![stablecoins]);

        // Init so we have config state.
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                security: "security".into(),
                stablecoin: "stablecoin".into(),
            },
        )
        .unwrap(); // panics on error

        // Onboard the trader (sets trader state, including loan cap).
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            ExecuteMsg::AddTrader {
                address: "trader".into(),
            },
        )
        .unwrap(); // panics on error

        // Buy some stocks without requiring loans.
        let funds = coin(100, "stablecoin");
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("trader", &[funds]),
            ExecuteMsg::BuyStock {
                amount: Uint128(100),
            },
        )
        .unwrap();

        // Ensure just one message was returned; a message to send stock to the trader.
        assert_eq!(res.messages.len(), 1);
        let security_amount = coin(100, "security"); // Note: assumes price is 1-to-1
        match unwrap_marker_params(&res.messages[0]) {
            MarkerMsgParams::WithdrawCoins {
                marker_denom,
                coin,
                recipient,
            } => {
                assert_eq!(marker_denom, "security");
                assert_eq!(coin, &security_amount);
                assert_eq!(recipient, &Addr::unchecked("trader"));
            }
            _ => panic!("expected marker withdraw params"),
        }
    }

    #[test]
    fn buy_with_loan() {
        // Create mocks.
        let mut deps = mock_dependencies(&[]);
        let stablecoins = coin(100, "stablecoin");
        deps.querier
            .base
            .update_balance("trader", vec![stablecoins]);

        // Init so we have config state.
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                security: "security".into(),
                stablecoin: "stablecoin".into(),
            },
        )
        .unwrap(); // panics on error

        // Onboard the trader (sets trader state, including loan cap).
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            ExecuteMsg::AddTrader {
                address: "trader".into(),
            },
        )
        .unwrap(); // panics on error

        // Buy 300 securities, but only send 100 stablecoin, requiring loans of 200 stablecoin.
        let funds = coin(100, "stablecoin");
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("trader", &[funds]),
            ExecuteMsg::BuyStock {
                amount: Uint128(300),
            },
        )
        .unwrap();

        // Ensure two messages were returned; one to take out the loan, one to send stock.
        assert_eq!(res.messages.len(), 2);

        // Assert expected amounts
        let expected_loan = coin(200, "stablecoin");
        let expected_security = coin(300, "security");
        res.messages
            .into_iter()
            .for_each(|msg| match unwrap_marker_params(&msg) {
                MarkerMsgParams::WithdrawCoins {
                    marker_denom,
                    coin,
                    recipient,
                } => {
                    if marker_denom == "security" {
                        assert_eq!(coin, &expected_security);
                        assert_eq!(recipient, &Addr::unchecked("trader"));
                    } else {
                        assert_eq!(marker_denom, "stablecoin");
                        assert_eq!(coin, &expected_loan);
                        assert_eq!(recipient, &Addr::unchecked(MOCK_CONTRACT_ADDR));
                    }
                }
                _ => panic!("expected marker withdraw params"),
            });

        // Query trader state
        let bin = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetTraderState {
                address: "trader".into(),
            },
        )
        .unwrap(); // panics on error

        // Ensure trader state has the expected amount of loans captured
        let rep: TraderStateResponse = from_binary(&bin).unwrap();
        assert_eq!(rep.loans, Uint128(200));
    }

    #[test]
    fn sell_with_proceeds() {
        // Create mocks.
        let mut deps = mock_dependencies(&[]);

        // Add expected markers to the mock querier
        let bin = must_read_binary_file("testdata/security.json");
        let security_marker: Marker = from_binary(&bin).unwrap();
        let bin = must_read_binary_file("testdata/stablecoin.json");
        let stablecoin_marker: Marker = from_binary(&bin).unwrap();
        deps.querier
            .with_markers(vec![security_marker, stablecoin_marker]);

        // Init so we have config state.
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                security: "security".into(),
                stablecoin: "stablecoin".into(),
            },
        )
        .unwrap(); // panics on error

        // Onboard the trader (sets trader state, including loan cap).
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            ExecuteMsg::AddTrader {
                address: "trader".into(),
            },
        )
        .unwrap(); // panics on error

        // Sell 100 securities, with zero trader loans to pay off.
        let funds = coin(100, "security");
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("trader", &[funds]),
            ExecuteMsg::SellStock {
                amount: Uint128(100),
            },
        )
        .unwrap();

        // Ensure two messages were returned; one to send stock to the pool, one to send stablecoin
        // to the trader.
        assert_eq!(res.messages.len(), 2);

        // Validate bank transfer addresses and amounts.
        res.messages.into_iter().for_each(|msg| match msg {
            CosmosMsg::Bank(BankMsg::Send {
                amount, to_address, ..
            }) => {
                assert_eq!(amount.len(), 1);
                if to_address == Addr::unchecked("trader") {
                    let expected_proceeds = coin(100, "stablecoin");
                    assert_eq!(amount[0], expected_proceeds);
                } else {
                    assert_eq!(to_address, "security");
                    let expected_security = coin(100, "security");
                    assert_eq!(amount[0], expected_security);
                }
            }
            _ => panic!("unexpected message type"),
        });
    }

    #[test]
    fn sell_with_loans() {
        // Create mocks.
        let mut deps = mock_dependencies(&[]);
        let stablecoins = coin(100, "stablecoin");
        deps.querier
            .base
            .update_balance("trader", vec![stablecoins]);

        // Add expected markers to the mock querier
        let bin = must_read_binary_file("testdata/security.json");
        let security_marker: Marker = from_binary(&bin).unwrap();
        let bin = must_read_binary_file("testdata/stablecoin.json");
        let stablecoin_marker: Marker = from_binary(&bin).unwrap();
        deps.querier
            .with_markers(vec![security_marker, stablecoin_marker]);

        // Init so we have config state.
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            InitMsg {
                security: "security".into(),
                stablecoin: "stablecoin".into(),
            },
        )
        .unwrap(); // panics on error

        // Onboard the trader (sets trader state, including loan cap).
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("admin", &[]),
            ExecuteMsg::AddTrader {
                address: "trader".into(),
            },
        )
        .unwrap(); // panics on error

        // Buy 300 securities, requiring loans of 200 stablecoin.
        let funds = coin(100, "stablecoin");
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("trader", &[funds]),
            ExecuteMsg::BuyStock {
                amount: Uint128(300),
            },
        )
        .unwrap();

        // Query trader state
        let bin = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetTraderState {
                address: "trader".into(),
            },
        )
        .unwrap(); // panics on error

        // Ensure trader state has the expected amount of loans captured
        let rep: TraderStateResponse = from_binary(&bin).unwrap();
        assert_eq!(rep.loans, Uint128(200));

        // Sell all 300 securities
        let funds = coin(300, "security");
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("trader", &[funds]),
            ExecuteMsg::SellStock {
                amount: Uint128(300),
            },
        )
        .unwrap();

        // Ensure three messages were returned; one to send stock to the security pool, one to send
        // stablecoin to the loan pool (loan payment), and net proceeds to the trader.
        assert_eq!(res.messages.len(), 3);

        // Validate bank transfer addresses and amounts.
        res.messages.into_iter().for_each(|msg| match msg {
            CosmosMsg::Bank(BankMsg::Send {
                amount, to_address, ..
            }) => {
                assert_eq!(amount.len(), 1);
                if to_address == Addr::unchecked("trader") {
                    let expected_proceeds = coin(100, "stablecoin");
                    assert_eq!(amount[0], expected_proceeds);
                } else if to_address == Addr::unchecked("stablecoin") {
                    let expected_loan_payment = coin(200, "stablecoin");
                    assert_eq!(amount[0], expected_loan_payment);
                } else {
                    assert_eq!(to_address, "security");
                    let expected_security = coin(300, "security");
                    assert_eq!(amount[0], expected_security);
                }
            }
            _ => panic!("unexpected message type"),
        });

        // Query trader state
        let bin = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetTraderState {
                address: "trader".into(),
            },
        )
        .unwrap(); // panics on error

        // Ensure trader state has the loans paid off
        let rep: TraderStateResponse = from_binary(&bin).unwrap();
        assert_eq!(rep.loans, Uint128::zero());
    }
}
