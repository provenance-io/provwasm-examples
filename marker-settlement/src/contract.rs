use cosmwasm_std::{
    Deps, DepsMut, Env, HandleResponse, InitResponse, MessageInfo, QueryResponse, StdError,
};
use provwasm_std::{bind_name, transfer_marker_coins, Marker, ProvenanceMsg, ProvenanceQuerier};

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{config, config_read, State};

// Initialize the contract, saving the instantiator as the contract owner.
pub fn init(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse<ProvenanceMsg>, ContractError> {
    // Funds should not be sent with the message for restricted markers.
    if !info.sent_funds.is_empty() {
        let errm = "funds sent during init";
        return Err(ContractError::Std(StdError::generic_err(errm)));
    }

    // Initialize and store configuration state.
    let state = State {
        exchange: info.sender, // The exchange must send a Wasm message to init this instance.
        denoms: msg.denoms,
    };
    config(deps.storage).save(&state)?;

    // Issue a message that will bind a restricted name to the contract address.
    let msg = bind_name(msg.contract_name, env.contract.address);
    Ok(InitResponse {
        messages: vec![msg],
        attributes: vec![],
    })
}

// Transfer funds using the marker module.
pub fn handle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse<ProvenanceMsg>, ContractError> {
    // Funds should not be sent with the message for restricted markers.
    if !info.sent_funds.is_empty() {
        let errm = "sending funds is not supported in restricted marker settlements";
        return Err(ContractError::Std(StdError::generic_err(errm)));
    }

    // Validate the message sender is the contact owner.
    let state = config_read(deps.storage).load()?;
    if info.sender != state.exchange {
        return Err(ContractError::Unauthorized {});
    }

    // Transfer funds using the marker module. NOTE: The contract must have transfer permission on
    // the restricted marker.
    match msg {
        HandleMsg::Settlement { coin, to, from } => {
            if !state.denoms.contains(&coin.denom) {
                let errm = format!("unsupported denom: {}", coin.denom);
                return Err(ContractError::Std(StdError::generic_err(errm)));
            }
            check_restricted_marker(deps.as_ref(), coin.denom.clone())?;
            let msg = transfer_marker_coins(coin, to, from);
            Ok(HandleResponse {
                messages: vec![msg],
                attributes: vec![],
                data: None,
            })
        }
    }
}

// Return an error if the given denom doesn't represent a restricted marker.
fn check_restricted_marker(deps: Deps, denom: String) -> Result<(), ContractError> {
    let querier = ProvenanceQuerier::new(&deps.querier);
    let marker: Marker = querier.get_marker_by_denom(denom)?;
    if marker.bank_sends_disabled() {
        return Ok(());
    }
    Err(ContractError::Std(StdError::generic_err(
        "marker must be restricted for this settlement contract",
    )))
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
        // Create default provenance mocks.
        let mut deps = mock_dependencies(&[]);

        // Call init
        let res = init(
            deps.as_mut(),
            mock_env(),
            mock_info("exchange", &[]),
            InitMsg {
                contract_name: "marker.settlement.pb".into(),
                denoms: vec!["tokens".into()],
            },
        )
        .unwrap();

        // Ensure bind name message was created.
        assert_eq!(1, res.messages.len());
    }

    #[test]
    fn valid_settlement() {
        // Create a test settlement amount
        let settlement_amount = coin(12345, "tokens");

        // Read the test marker from file
        let bin = must_read_binary_file("testdata/marker.json");
        let marker: Marker = from_binary(&bin).unwrap();

        // Create provenance mocks with the marker and settlement amount.
        let mut deps = mock_dependencies(&[]);
        deps.querier.with_markers(vec![marker.clone()]);
        let info = mock_info("exchange", &[]);

        // Call init so we have state
        init(
            deps.as_mut(),
            mock_env(),
            mock_info("exchange", &[]),
            InitMsg {
                contract_name: "marker.settlement.pb".into(),
                denoms: vec!["tokens".into()],
            },
        )
        .unwrap();

        // Handle a settlement to send funds to a bidder.
        let msg = HandleMsg::Settlement {
            coin: settlement_amount.clone(),
            to: HumanAddr::from("ask"),
            from: HumanAddr::from("bid"),
        };
        let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

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
