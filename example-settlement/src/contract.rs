use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse,
    MessageInfo, QueryResponse, StdError,
};
use provwasm_std::{
    bind_name, transfer_marker_coins, MarkerType, ProvenanceMsg, ProvenanceQuerier,
};

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};

/// Initialize the contract configuration state and bind a name to the contract instance.
pub fn init(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse<ProvenanceMsg>, ContractError> {
    // Funds should NOT be sent to init.
    if !info.sent_funds.is_empty() {
        return Err(generic_err("funds sent during init"));
    }

    // Initialize and store configuration state.
    let state = State { admin: info.sender };
    config(deps.storage).save(&state)?;

    // Issue a message that will bind a restricted name to the contract address.
    let msg = bind_name(msg.contract_name, env.contract.address);
    Ok(InitResponse {
        messages: vec![msg],
        attributes: vec![],
    })
}

/// Transfer settlement funds.
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse<ProvenanceMsg>, ContractError> {
    // Funds should NOT be sent to handle.
    if !info.sent_funds.is_empty() {
        return Err(generic_err("funds sent during handle"));
    }

    // Load contract state
    let state = config_read(deps.storage).load()?;

    // Validate the message sender is the contact admin.
    if info.sender != state.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Dispatch settlment transfers to the appropriate actors.
    match msg {
        // In a "real" exchange, we'd look up the bid and ask from storage and validate them.
        // This is just a demo of how to dispatch settlement transfers.
        HandleMsg::Settlement {
            asker,
            ask,
            bidder,
            bid,
        } => {
            if asker == bidder {
                return Err(generic_err("bidder equals asker"));
            }
            let deps_ref = deps.as_ref();
            let transfer1 = transfer_or_send(deps_ref, &env, &bid, &asker, &bidder);
            let transfer2 = transfer_or_send(deps_ref, &env, &ask, &bidder, &asker);
            Ok(HandleResponse {
                messages: vec![transfer1, transfer2],
                attributes: vec![],
                data: None,
            })
        }
    }
}

fn transfer_or_send(
    deps: Deps,
    env: &Env,
    amount: &Coin,
    to: &HumanAddr,
    from: &HumanAddr,
) -> CosmosMsg<ProvenanceMsg> {
    if requires_marker_transfer(deps, &amount.denom) {
        transfer_marker_coins(amount.clone(), to.clone(), from.clone())
    } else {
        CosmosMsg::Bank(BankMsg::Send {
            to_address: to.clone(),
            from_address: env.contract.address.clone(), // Bank transfers require escrowed funds
            amount: vec![amount.clone()],
        })
    }
}

// Returns true iff a denom is backed by a restricted marker
fn requires_marker_transfer(deps: Deps, denom: &str) -> bool {
    let querier = ProvenanceQuerier::new(&deps.querier);
    match querier.get_marker_by_denom(denom.into()) {
        Ok(marker) => matches!(marker.marker_type, MarkerType::Restricted),
        Err(_) => false,
    }
}

// An error helper function
fn generic_err(errm: &str) -> ContractError {
    ContractError::Std(StdError::generic_err(errm))
}

/// Query does nothing
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> Result<QueryResponse, StdError> {
    Ok(QueryResponse::default())
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use cosmwasm_std::testing::{mock_env, mock_info};
    // use cosmwasm_std::{coin, from_binary, HumanAddr};
    // use provwasm_mocks::{mock_dependencies, must_read_binary_file};
    // use provwasm_std::Marker;

    #[test]
    fn valid_init() {
        todo!()
    }

    #[test]
    fn valid_settlement() {
        todo!()
    }
}
