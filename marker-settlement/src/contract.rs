use cosmwasm_std::{
    Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo, QueryResponse,
    StdError,
};
use provwasm_std::{
    bind_name, transfer_marker_coins, MarkerType, ProvenanceMsg, ProvenanceQuerier,
};

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};

// Initialize the contract configuration state and bind a name to the contract instance.
pub fn init(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse<ProvenanceMsg>, ContractError> {
    // Funds should NOT be sent to restricted marker settlement instances.
    if !info.sent_funds.is_empty() {
        return Err(generic_err("funds sent during init"));
    }

    // Ensure at least one denomination was sent.
    if msg.denoms.is_empty() {
        return Err(generic_err("no denominations provided during init"));
    }

    // Ensure all sent denominations are backed by restricted markers.
    for denom in msg.denoms.iter() {
        ensure_restricted_marker(deps.as_ref(), denom)?;
    }

    // Initialize and store configuration state.
    // NOTE: The exchange can also be the admin by instantiating the instance itself.
    let state = State {
        admin: info.sender,
        exchange: msg.exchange,
        denoms: msg.denoms,
        attrs: msg.attrs,
    };
    config(deps.storage).save(&state)?;

    // Issue a message that will bind a restricted name to the contract address.
    let msg = bind_name(msg.contract_name, env.contract.address);
    Ok(InitResponse {
        messages: vec![msg],
        attributes: vec![],
    })
}

// Transfer funds backed by restricted markers using the marker module.
pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse<ProvenanceMsg>, ContractError> {
    // Funds should NOT be sent with the message.
    if !info.sent_funds.is_empty() {
        return Err(generic_err("sending funds is not supported"));
    }

    // Validate the message sender is the exchange or the contact admin.
    let state = config_read(deps.storage).load()?;
    if info.sender != state.exchange && info.sender != state.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Transfer funds using the marker module.
    // NOTE: This contract instance must have 'transfer' permission on the restricted marker.
    match msg {
        HandleMsg::Settlement { coin, to, from } => {
            // Ensure we got a supported denom
            if !state.denoms.contains(&coin.denom) {
                let errm = format!("unsupported denom: {}", coin.denom);
                return Err(generic_err(&errm));
            }
            // Double check that the denom is backed by a restricted marker.
            ensure_restricted_marker(deps.as_ref(), &coin.denom)?;
            // Ensure recpient has all required attributes before we transfer.
            ensure_recipient_attributes(deps.as_ref(), to.clone(), state.attrs)?;
            // Dispatch transfer params to the marker module transfer handler.
            let msg = transfer_marker_coins(coin, to, from);
            Ok(HandleResponse {
                messages: vec![msg],
                attributes: vec![],
                data: None,
            })
        }
    }
}

// Return an error if the given denom is NOT backed by a restricted marker.
fn ensure_restricted_marker(deps: Deps, denom: &str) -> Result<(), ContractError> {
    if !requires_marker_transfer(deps, denom) {
        return Err(generic_err("restricted markers are required"));
    }
    Ok(())
}

// Returns true iff a denom is backed by a restricted marker
fn requires_marker_transfer(deps: Deps, denom: &str) -> bool {
    let querier = ProvenanceQuerier::new(&deps.querier);
    match querier.get_marker_by_denom(denom.into()) {
        Ok(marker) => matches!(marker.marker_type, MarkerType::Restricted),
        Err(_) => false,
    }
}

// Return an error if a transfer recipient doesn't have all the given attributes
fn ensure_recipient_attributes(
    deps: Deps,
    to: HumanAddr,
    attrs: Vec<String>,
) -> Result<(), ContractError> {
    // Skip the check if no attributes are required.
    if attrs.is_empty() {
        return Ok(());
    }
    // Check for all provided attributes
    let querier = ProvenanceQuerier::new(&deps.querier);
    for name in attrs.iter() {
        let res = querier.get_attributes(to.clone(), Some(name.clone()))?;
        if res.attributes.is_empty() {
            let errm = format!("named attribute {} not found for {}", name.clone(), to);
            return Err(generic_err(&errm));
        }
    }
    Ok(())
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
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{coin, from_binary, CosmosMsg, HumanAddr};
    use provwasm_mocks::{mock_dependencies, must_read_binary_file};
    use provwasm_std::{Marker, MarkerMsgParams, ProvenanceMsgParams};

    #[test]
    fn valid_init() {
        // Read the test marker from file
        let bin = must_read_binary_file("testdata/marker.json");
        let marker: Marker = from_binary(&bin).unwrap();
        assert_eq!(marker.marker_type, MarkerType::Restricted);

        // Create provenance mocks.
        let mut deps = mock_dependencies(&[]);
        deps.querier.with_markers(vec![marker.clone()]);

        // Call init
        let res = init(
            deps.as_mut(),
            mock_env(),
            mock_info("exchange", &[]),
            InitMsg {
                exchange: HumanAddr::from("exchange"),
                contract_name: "restricted.settlement.sc.pb".into(),
                denoms: vec!["tokens".into()],
                attrs: vec![],
            },
        )
        .unwrap();

        // Ensure bind name message was created.
        assert_eq!(1, res.messages.len());
    }

    #[test]
    fn valid_restricted_marker_settlement() {
        // Read the test marker from file
        let bin = must_read_binary_file("testdata/marker.json");
        let marker: Marker = from_binary(&bin).unwrap();
        assert_eq!(marker.marker_type, MarkerType::Restricted);

        // Create provenance mocks with the marker and settlement amount.
        let mut deps = mock_dependencies(&[]);
        deps.querier.with_markers(vec![marker.clone()]);
        let info = mock_info("exchange", &[]);

        // Create a test settlement amount
        let settlement_amount = coin(12345, "tokens");

        // Call init so we have state
        init(
            deps.as_mut(),
            mock_env(),
            mock_info("exchange", &[]),
            InitMsg {
                exchange: HumanAddr::from("exchange"),
                contract_name: "restricted.settlement.sc.pb".into(),
                denoms: vec!["tokens".into()],
                attrs: vec![],
            },
        )
        .unwrap();

        // Handle a settlement to send funds to a bidder.
        let res = handle(
            deps.as_mut(),
            mock_env(),
            info,
            HandleMsg::Settlement {
                coin: settlement_amount.clone(),
                to: HumanAddr::from("ask"),
                from: HumanAddr::from("bid"),
            },
        )
        .unwrap();

        // Check we got a single set of marker transfer params
        assert_eq!(res.messages.len(), 1);

        // Assert the correct params were created
        match unwrap_marker_params(&res.messages[0]) {
            MarkerMsgParams::TransferMarkerCoins { coin, to, from } => {
                assert_eq!(*coin, settlement_amount);
                assert_eq!(*to, HumanAddr::from("ask"));
                assert_eq!(*from, HumanAddr::from("bid"));
            }
            _ => panic!("expected marker transfer params"),
        }
    }

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
}
