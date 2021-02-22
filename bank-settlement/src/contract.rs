use cosmwasm_std::{
    BankMsg, CosmosMsg, Deps, DepsMut, Env, HandleResponse, HumanAddr, InitResponse, MessageInfo,
    QueryResponse, StdError,
};
use provwasm_std::{bind_name, MarkerType, ProvenanceMsg, ProvenanceQuerier};

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
    // Funds should NOT be sent during init.
    if !info.sent_funds.is_empty() {
        return Err(generic_err("funds sent during init"));
    }

    // Ensure at least one denomination was sent.
    if msg.denoms.is_empty() {
        return Err(generic_err("no denominations provided during init"));
    }

    // Ensure all sent denominations can use bank sends.
    for denom in msg.denoms.iter() {
        ensure_bank_send(deps.as_ref(), denom)?;
    }

    // Initialize and store configuration state.
    // NOTE: The exchange can also be the admin by instantiating the code itself.
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

// Transfer funds using the bank module.
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse, ContractError> {
    // Funds MUST be sent with the message for bank transfers to work.
    if info.sent_funds.is_empty() {
        return Err(generic_err("funds are required for bank settlements"));
    }

    // Validate the message sender has permission.
    let state = config_read(deps.storage).load()?;
    if info.sender != state.exchange && info.sender != state.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Ensure the funds are non-zero and have a supported denomination.
    for funds in info.sent_funds.iter() {
        if funds.amount.is_zero() || !state.denoms.contains(&funds.denom) {
            let errm = format!("invalid settlement funds: {}{}", funds.amount, funds.denom);
            return Err(generic_err(&errm));
        }
        ensure_bank_send(deps.as_ref(), &funds.denom)?;
    }

    // Transfer funds using the bank module.
    match msg {
        HandleMsg::Settlement { to } => {
            // Ensure recpient has all required attributes before transfer.
            ensure_recipient_attributes(deps.as_ref(), to.clone(), state.attrs)?;
            // Create a bank send
            let msg = CosmosMsg::Bank(BankMsg::Send {
                from_address: env.contract.address,
                to_address: to,
                amount: info.sent_funds,
            });
            // Dispatch it to the bank module
            Ok(HandleResponse {
                messages: vec![msg],
                attributes: vec![],
                data: None,
            })
        }
    }
}

// Return an error if the given denom is backed by a restricted marker.
fn ensure_bank_send(deps: Deps, denom: &str) -> Result<(), ContractError> {
    if requires_marker_transfer(deps, denom) {
        let errm = format!("bank sends disabled for denom {}", denom);
        return Err(generic_err(&errm));
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
    use cosmwasm_std::{coin, from_binary, HumanAddr};
    use provwasm_mocks::{mock_dependencies, must_read_binary_file};
    use provwasm_std::Marker;

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
                exchange: HumanAddr::from("exchange"),
                contract_name: "bank.settlement.sc.pb".into(),
                denoms: vec!["tokens".into()],
                attrs: vec![],
            },
        )
        .unwrap();

        // Ensure bind name message was created.
        assert_eq!(1, res.messages.len());
    }

    // Make sure bank settlements work with unrestricted markers
    #[test]
    fn valid_unrestricted_marker_settlement() {
        // Create a test settlement amount
        let settlement_amount = coin(12345, "tokens");

        // Read the test marker from file
        let bin = must_read_binary_file("testdata/marker.json");
        let marker: Marker = from_binary(&bin).unwrap();

        // Create provenance mocks with the marker and settlement amount.
        let mut deps = mock_dependencies(&[]);
        deps.querier.with_markers(vec![marker.clone()]);
        let info = mock_info("exchange", &[settlement_amount.clone()]);

        // Call init so we have state
        init(
            deps.as_mut(),
            mock_env(),
            mock_info("exchange", &[]),
            InitMsg {
                exchange: HumanAddr::from("exchange"),
                contract_name: "bank.settlement.sc.pb".into(),
                denoms: vec!["tokens".into()],
                attrs: vec![],
            },
        )
        .unwrap();

        // Handle a settlement to send funds via bank module.
        let res = handle(
            deps.as_mut(),
            mock_env(),
            info,
            HandleMsg::Settlement {
                to: HumanAddr::from("ask"),
            },
        )
        .unwrap();

        // Check we got a single set of bank send params
        assert_eq!(res.messages.len(), 1);

        // Ensure the expected bank message was dispatched.
        res.messages.into_iter().for_each(|bmsg| match bmsg {
            CosmosMsg::Bank(BankMsg::Send {
                amount, to_address, ..
            }) => {
                assert_eq!(amount.len(), 1);
                assert_eq!(amount[0], settlement_amount);
                assert_eq!(*to_address, HumanAddr::from("ask"));
            }
            _ => panic!("unexpected message type"),
        });
    }

    // Make sure bank settlements work with coins not backed by a marker.
    #[test]
    fn valid_coin_settlement() {
        // Create a test settlement amount
        let settlement_amount = coin(12345, "tokens");

        // Create provenance mocks with the marker and settlement amount.
        let mut deps = mock_dependencies(&[]);
        let info = mock_info("exchange", &[settlement_amount.clone()]);

        // Call init so we have state
        init(
            deps.as_mut(),
            mock_env(),
            mock_info("exchange", &[]),
            InitMsg {
                exchange: HumanAddr::from("exchange"),
                contract_name: "bank.settlement.pb".into(),
                denoms: vec!["tokens".into()],
                attrs: vec![],
            },
        )
        .unwrap();

        // Handle a settlement to send funds via bank module.
        let res = handle(
            deps.as_mut(),
            mock_env(),
            info,
            HandleMsg::Settlement {
                to: HumanAddr::from("ask"),
            },
        )
        .unwrap();

        // Check we got a single set of bank send params
        assert_eq!(res.messages.len(), 1);

        // Ensure the expected bank message was dispatched.
        res.messages.into_iter().for_each(|msg| match msg {
            CosmosMsg::Bank(BankMsg::Send {
                amount, to_address, ..
            }) => {
                assert_eq!(amount.len(), 1);
                assert_eq!(amount[0], settlement_amount);
                assert_eq!(*to_address, HumanAddr::from("ask"));
            }
            _ => panic!("unexpected message type"),
        });
    }
}
