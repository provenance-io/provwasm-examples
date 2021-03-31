use cosmwasm_std::{
    coin, has_coins, to_binary, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, HumanAddr,
    MessageInfo, QueryResponse, Response, StdError, Uint128,
};

use provwasm_std::{
    grant_marker_access, transfer_marker_coins, withdraw_coins, MarkerAccess, ProvenanceMsg,
    ProvenanceQuerier,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InitMsg, QueryMsg, TraderStateResponse};
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
    address: HumanAddr,
) -> Result<Response<ProvenanceMsg>, ContractError> {
    // Load contract state and validate the message sender is the contact admin.
    let state = config_read(deps.storage).load()?;
    if info.sender != state.contract_admin {
        return Err(ContractError::Unauthorized {});
    }

    // Query trader's stablecoin balance, ensuring it is non-zero.
    let balance: Coin = deps.querier.query_balance(&address, &state.stablecoin)?;
    if balance.amount.is_zero() {
        return Err(ContractError::InsufficientFunds {});
    }

    // Load trader config bucket
    let mut bucket = trader_bucket(deps.storage);

    // Initialize and save trader config state if necessary.
    let trader_key = deps.api.canonical_address(&address)?;
    if bucket.may_load(&trader_key)?.is_none() {
        bucket.save(
            &trader_key,
            &TraderState {
                loan_cap: Uint128(9 * balance.amount.u128()),
                loans: Uint128::zero(),
            },
        )?;
    }

    // Hmmm, in order for trader to sell shares, they need transfer permission on the restricted
    // security marker.
    let grant_msg = grant_marker_access(&state.security, &address, vec![MarkerAccess::Transfer])?;

    // Add grant to response.
    let mut res = Response::new();
    res.add_message(grant_msg);
    Ok(res)
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
    let trader_key = deps.api.canonical_address(&info.sender)?;
    let trader_state = trader_bucket_read(deps.storage).load(&trader_key)?;
    if info.funds.is_empty() && trader_state.loans >= trader_state.loan_cap {
        return Err(ContractError::LoanCapExceeded {});
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
        let loan_amount = (price.amount - sent_amount)?;

        // Ensure trader is under loan cap after borrowing.
        let max_loan_amount = (trader_state.loan_cap - trader_state.loans)?;
        if loan_amount > max_loan_amount {
            return Err(ContractError::LoanCapExceeded {});
        }

        // Escrow loan amount from the stablecoin loan pool marker into the contract
        let loan_msg = withdraw_coins(
            stablecoin,
            loan_amount.u128(),
            stablecoin,
            &env.contract.address,
        )?;
        res.add_message(loan_msg);

        // Update the trader's loan total
        trader_bucket(deps.storage).update(&trader_key, |opt| -> Result<_, ContractError> {
            match opt {
                Some(mut ts) => {
                    ts.loans += loan_amount;
                    Ok(ts)
                }
                None => Err(ContractError::UnknownTrader {}),
            }
        })?;
    }

    // TODO: What to do if trader sent more than required - error?
    let stock_msg = withdraw_coins(security, amount.u128(), security, &info.sender)?;
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
    // Ensure no funds are sent for sells.
    if amount.is_zero() || !info.funds.is_empty() {
        return Err(ContractError::InvalidSell {});
    }

    // Load trader state
    let sender = &info.sender;
    let trader_key = deps.api.canonical_address(sender)?;
    let trader_state = trader_bucket_read(deps.storage).load(&trader_key)?;

    // Load security and stablecoin marker denoms.
    let config_state = config_read(deps.storage).load()?;
    let security: &str = &config_state.security;
    let security_pool: HumanAddr = get_marker_address(deps.as_ref(), security)?;
    let stablecoin: &str = &config_state.stablecoin;
    let stablecoin_pool: HumanAddr = get_marker_address(deps.as_ref(), stablecoin)?;

    // Ensure sender has the requested shares in their account
    let balance: Coin = deps.querier.query_balance(&info.sender, security)?;
    if balance.amount < amount {
        return Err(ContractError::InsufficientFunds {});
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
            to_address: info.sender.clone(),
        });
        res.add_message(bank_msg);

    // Trader needs to pay back loans, but gets some stablecoin from the sale.
    } else if proceeds.amount > trader_state.loans {
        // Send the entire loan amount back to the loan pool
        let loan_amount = coin(trader_state.loans.u128(), stablecoin);
        let loan_msg: CosmosMsg<ProvenanceMsg> = CosmosMsg::Bank(BankMsg::Send {
            amount: vec![loan_amount],
            to_address: stablecoin_pool,
        });
        res.add_message(loan_msg);

        // Determine the amount to send to the trader
        let net = (proceeds.amount - trader_state.loans)?;
        let net_amount = coin(net.u128(), stablecoin);
        let net_msg: CosmosMsg<ProvenanceMsg> = CosmosMsg::Bank(BankMsg::Send {
            amount: vec![net_amount],
            to_address: info.sender.clone(),
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
            to_address: stablecoin_pool,
        });
        res.add_message(bank_msg);

        // Mark off trader loan debt.
        let updated_loans = (trader_state.loans - proceeds.amount)?;
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

    // XXX: Bug here - this won't work because x/wasm needs ALL signers to be the contract.
    // Right now, signers are set as contract and sender in the marker module, causing the
    // following error:
    //
    //    failed to execute message; message index: 0: dispatch: contract doesn't have permission:
    //    unauthorized: invalid request

    // Tansfer security back to stock pool
    let transfer_msg = transfer_marker_coins(amount.u128(), security, &security_pool, sender)?;
    res.add_message(transfer_msg);

    // Not working ^

    Ok(res)
}

// Return an error if the given denoms are NOT backed by markers.
fn get_marker_address(deps: Deps, denom: &str) -> Result<HumanAddr, ContractError> {
    let querier = ProvenanceQuerier::new(&deps.querier);
    let marker = querier.get_marker_by_denom(denom)?;
    Ok(marker.address)
}

/// Handle query requests for trader loans
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<QueryResponse, StdError> {
    match msg {
        QueryMsg::GetTraderState { address } => try_get_trader_state(deps, address),
    }
}

// Query for trader loan cap and debt.
fn try_get_trader_state(deps: Deps, address: HumanAddr) -> Result<QueryResponse, StdError> {
    // Load trader state
    let trader_key = deps.api.canonical_address(&address)?;
    let trader_state = trader_bucket_read(deps.storage).load(&trader_key)?;
    // Load config state
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

#[cfg(test)]
mod tests {
    // use super::*;
    // use cosmwasm_std::testing::{mock_env, mock_info};
    // use cosmwasm_std::{from_binary, CosmosMsg, StdError};
    // use provwasm_mocks::mock_dependencies;
    // use provwasm_std::{NameMsgParams, Names, ProvenanceMsgParams};

    #[test]
    fn init_test() {
        // TODO
    }

    #[test]
    fn buy_test() {
        // TODO
    }

    #[test]
    fn sell_test() {
        // TODO
    }

    #[test]
    fn query_test() {
        // TODO
    }
}
