use cosmwasm_std::{
    to_binary, Coin, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo,
    QueryResponse, StdError, WasmMsg,
};
use provwasm_std::{bind_name, MarkerType, Name, ProvenanceMsg, ProvenanceQuerier};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};

// Message dispatched to the bank settlement actor.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BankActor {
    Settlement { to: HumanAddr },
}

// Message dispatched to the marker settlement actor.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MarkerActor {
    Settlement {
        coin: Coin,
        to: HumanAddr,
        from: HumanAddr,
    },
}

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

    // Ensure the helper settlement contract names are bound.
    let deps_ref = deps.as_ref();
    resolve_name(deps_ref, msg.bank_settlement_name.clone())?;
    resolve_name(deps_ref, msg.marker_settlement_name.clone())?;

    // Initialize and store configuration state.
    let state = State {
        admin: info.sender,
        bank_settlement: msg.bank_settlement_name,
        marker_settlement: msg.marker_settlement_name,
    };
    config(deps.storage).save(&state)?;

    // Issue a message that will bind a restricted name to the contract address.
    let msg = bind_name(msg.contract_name, env.contract.address);
    Ok(InitResponse {
        messages: vec![msg],
        attributes: vec![],
    })
}

// Return an error if a name does not resolve to an address.
fn resolve_name(deps: Deps, name: String) -> Result<HumanAddr, ContractError> {
    let querier = ProvenanceQuerier::new(&deps.querier);
    let name: Name = querier.resolve_name(name)?;
    Ok(name.address)
}

/// Transfer funds using settlement actors.
pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    // Funds should NOT be sent to handle.
    if !info.sent_funds.is_empty() {
        return Err(generic_err("funds sent during handle"));
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
            // Ensure bidder is not asker
            if bidder == asker {
                return Err(generic_err("bidder cannot equal asker"));
            }
            // Build wasm messages
            let deps_ref = deps.as_ref();
            // Settlement transfer of bid amount to asker from bidder
            let msg1 = wasm_transfer(deps_ref, bid, asker.clone(), bidder.clone())?;
            // Settlement transfer of ask amount to bidder from asker
            let msg2 = wasm_transfer(deps_ref, ask, bidder.clone(), asker.clone())?;
            // Dispatch to the appropriate settlement actors
            Ok(HandleResponse {
                messages: vec![msg1.into(), msg2.into()],
                attributes: vec![],
                data: None,
            })
        }
    }
}

// Build a transfer message to either the bank or marker settlment actors.
fn wasm_transfer(
    deps: Deps,
    coin: Coin,
    to: HumanAddr,
    from: HumanAddr,
) -> Result<WasmMsg, ContractError> {
    if requires_marker_transfer(deps, &coin.denom) {
        wasm_marker_transfer(deps, coin, to, from)
    } else {
        wasm_bank_transfer(deps, coin, to)
    }
}

// Create a message that will be sent to the marker settlement actor.
fn wasm_marker_transfer(
    deps: Deps,
    coin: Coin,
    to: HumanAddr,
    from: HumanAddr,
) -> Result<WasmMsg, ContractError> {
    let state = config_read(deps.storage).load()?;
    let marker_settlement_address = resolve_name(deps, state.marker_settlement)?;
    let settlement = MarkerActor::Settlement { coin, to, from };
    let bin = to_binary(&settlement)?;
    Ok(WasmMsg::Execute {
        contract_addr: marker_settlement_address,
        msg: bin,
        send: vec![], // NOTE: restricted marker funds should NOT be escrowed
    })
}

// Create a message that will be sent to the bank settlement actor.
fn wasm_bank_transfer(deps: Deps, coin: Coin, to: HumanAddr) -> Result<WasmMsg, ContractError> {
    let state = config_read(deps.storage).load()?;
    let bank_settlement_address = resolve_name(deps, state.bank_settlement)?;
    let settlement = BankActor::Settlement { to };
    let bin = to_binary(&settlement)?;
    Ok(WasmMsg::Execute {
        contract_addr: bank_settlement_address,
        msg: bin,
        send: vec![coin], // NOTE: bank transfer amounts MUST be escrowed
    })
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
