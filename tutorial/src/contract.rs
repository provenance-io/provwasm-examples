use cosmwasm_std::{
    attr, coin, to_binary, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, HandleResponse,
    InitResponse, MessageInfo, MigrateResponse, StdError, StdResult,
};
use provwasm_std::{bind_name, ProvenanceMsg};
use std::ops::Mul;

use crate::error::ContractError;
use crate::msg::{HandleMsg, InitMsg, MigrateMsg, QueryMsg};
use crate::state::{config, config_read, State};

/// Initialize the contract
pub fn init(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InitMsg,
) -> Result<InitResponse<ProvenanceMsg>, StdError> {
    // Ensure no funds were sent with the message
    if !info.sent_funds.is_empty() {
        let errm = "purchase funds are not allowed to be sent during init";
        return Err(StdError::generic_err(errm));
    }

    // Ensure there are limits on fees.
    if msg.fee_percent.is_zero() || msg.fee_percent > Decimal::percent(15) {
        return Err(StdError::generic_err(
            "fee percent must be > 0.0 and <= 0.15",
        ));
    }

    // Ensure the merchant address is not also the fee collection address
    if msg.merchant_address == info.sender {
        return Err(StdError::generic_err(
            "merchant address can't be the fee collection address",
        ));
    }

    // Create and save contract config state. The fee collection address represents the network
    // (ie they get paid fees), thus they must be the message sender.
    let state = State {
        contract_name: msg.contract_name.clone(),
        purchase_denom: msg.purchase_denom,
        merchant_address: msg.merchant_address,
        fee_collection_address: info.sender,
        fee_percent: msg.fee_percent,
    };
    config(deps.storage).save(&state)?;

    // Issue a message that will bind a restricted name to the contract address and emit an event.
    let bind_name_msg = bind_name(msg.contract_name, env.contract.address);
    Ok(InitResponse {
        messages: vec![bind_name_msg],
        attributes: vec![attr("tutorial-v2", ""), attr("action", "init")],
    })
}

/// Handle purchase messages.
pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: HandleMsg,
) -> Result<HandleResponse<BankMsg>, ContractError> {
    match msg {
        HandleMsg::Purchase { id } => try_purchase(deps, env, info, id),
    }
}

// Calculates transfers and fees, then dispatches messages to the bank module.
fn try_purchase(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<HandleResponse<BankMsg>, ContractError> {
    // Ensure funds were sent with the message
    if info.sent_funds.is_empty() {
        let errm = "no purchase funds sent";
        return Err(ContractError::Std(StdError::generic_err(errm)));
    }

    // Load state
    let state = config_read(deps.storage).load()?;
    let fee_pct = state.fee_percent;

    // Ensure the funds have the required amount and denomination
    for funds in info.sent_funds.iter() {
        if funds.amount.is_zero() || funds.denom != state.purchase_denom {
            let errm = format!("invalid purchase funds: {}{}", funds.amount, funds.denom);
            return Err(ContractError::Std(StdError::generic_err(errm)));
        }
    }

    // Calculate amounts and create bank transfers to the merchant account
    let transfers = CosmosMsg::Bank(BankMsg::Send {
        from_address: env.contract.address.clone(),
        to_address: state.merchant_address,
        amount: info
            .sent_funds
            .iter()
            .map(|sent| {
                let fees = sent.amount.mul(fee_pct).u128();
                coin(sent.amount.u128() - fees, sent.denom.clone())
            })
            .collect(),
    });

    // Calculate fees and create bank transfers to the fee collection account
    let fees = CosmosMsg::Bank(BankMsg::Send {
        from_address: env.contract.address,
        to_address: state.fee_collection_address,
        amount: info
            .sent_funds
            .iter()
            .map(|sent| coin(sent.amount.mul(fee_pct).u128(), sent.denom.clone()))
            .collect(),
    });

    // Return a response that will dispatch the transfers to the bank module and emit events.
    Ok(HandleResponse {
        messages: vec![transfers, fees],
        attributes: vec![
            attr("tutorial-v2", ""),
            attr("action", "purchase"),
            attr("purchase_id", id),
            attr("purchase_time", env.block.time), // Use BFT time as event timestamp
        ],
        data: None,
    })
}

/// Query for contract state.
pub fn query(
    deps: Deps,
    _env: Env, // NOTE: A '_' prefix indicates a variable is unused (supress linter warnings)
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryRequest {} => {
            let state = config_read(deps.storage).load()?;
            let json = to_binary(&state)?;
            Ok(json)
        }
    }
}

/// Called when migrating a contract instance to a new code ID.
pub fn migrate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: MigrateMsg,
) -> Result<MigrateResponse, ContractError> {
    // Ensure the updated fee percentage is within the new range.
    if msg.new_fee_percent.is_zero() || msg.new_fee_percent > Decimal::percent(15) {
        let errm = "fee percent must be > 0.0 and <= 0.15";
        return Err(ContractError::Std(StdError::generic_err(errm)));
    }

    // Get mutable state, ensure the message sender is the fee collector, and update fees.
    config(deps.storage).update(|mut state| {
        if info.sender != state.fee_collection_address {
            return Err(ContractError::Unauthorized {});
        }
        state.fee_percent = msg.new_fee_percent;
        Ok(state)
    })?;

    // Return the default success response
    Ok(MigrateResponse::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::QueryResponse;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{from_binary, HumanAddr};
    use provwasm_mocks::mock_dependencies;

    #[test]
    fn valid_init() {
        // Create mocks
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("feebucket", &[]);

        // Create a valid init message
        let msg = InitMsg {
            contract_name: "tutorial.sc.pb".into(),
            purchase_denom: "pcoin".into(),
            merchant_address: HumanAddr::from("merchant"),
            fee_percent: Decimal::percent(10),
        };

        // Ensure a message was created to bind the name to the contract address.
        let res = init(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.messages.len(), 1);
    }

    #[test]
    fn invalid_merchant_init() {
        // Create mocks
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("merchant", &[]); // error: merchant cannot be fee recipient

        // Create an invalid init message
        let msg = InitMsg {
            contract_name: "tutorial.sc.pb".into(),
            purchase_denom: "pcoin".into(),
            merchant_address: HumanAddr::from("merchant"),
            fee_percent: Decimal::percent(10),
        };

        // Ensure the expected error was returned.
        let err = init(deps.as_mut(), env, info, msg).unwrap_err();
        match err {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(msg, "merchant address can't be the fee collection address")
            }
            _ => panic!("unexpected init error"),
        }
    }

    #[test]
    fn invalid_fee_percent_init() {
        // Create mocks
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("feebucket", &[]);

        // Create an invalid init message
        let msg = InitMsg {
            contract_name: "tutorial.sc.pb".into(),
            purchase_denom: "pcoin".into(),
            merchant_address: HumanAddr::from("merchant"),
            fee_percent: Decimal::percent(37), // error: > 15%
        };

        // Ensure the expected error was returned.
        let err = init(deps.as_mut(), env, info, msg).unwrap_err();
        match err {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(msg, "fee percent must be > 0.0 and <= 0.15")
            }
            _ => panic!("unexpected init error"),
        }
    }

    #[test]
    fn query_test() {
        // Create mocks
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("feebucket", &[]);

        // Create a valid init message
        let msg = InitMsg {
            contract_name: "tutorial.sc.pb".into(),
            purchase_denom: "pcoin".into(),
            merchant_address: HumanAddr::from("merchant"),
            fee_percent: Decimal::percent(10),
        };
        let _ = init(deps.as_mut(), env, info, msg).unwrap(); // Panics on error

        // Call the smart contract query function to get stored state.
        let msg = QueryMsg::QueryRequest {};
        let bin = query(deps.as_ref(), mock_env(), msg).unwrap();
        let resp: QueryResponse = from_binary(&bin).unwrap();

        // Ensure the expected init fields were properly stored.
        assert_eq!(resp.contract_name, "tutorial.sc.pb");
        assert_eq!(resp.merchant_address, HumanAddr::from("merchant"));
        assert_eq!(resp.purchase_denom, "pcoin");
        assert_eq!(resp.fee_collection_address, HumanAddr::from("feebucket"));
        assert_eq!(resp.fee_percent, Decimal::percent(10));
    }

    #[test]
    fn handle_valid_purchase() {
        // Init contract state
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("feebucket", &[]);
        let msg = InitMsg {
            contract_name: "tutorial.sc.pb".into(),
            purchase_denom: "pcoin".into(),
            merchant_address: HumanAddr::from("merchant"),
            fee_percent: Decimal::percent(10),
        };
        let _ = init(deps.as_mut(), env, info, msg).unwrap();

        // Send a valid purchase message of 100pcoin
        let msg = HandleMsg::Purchase {
            id: "a7918172-ac09-43f6-bc4b-7ac2fbad17e9".into(),
        };
        let info = mock_info("consumer", &[coin(100, "pcoin")]);
        let res = handle(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Ensure we have the merchant transfer and fee collection bank messages
        assert_eq!(res.messages.len(), 2);

        // Ensure we got the proper bank transfer values.
        // 10% fees on 100 pcoin => 90 pcoin for the merchant and 10 pcoin for the fee bucket.
        let expected_transfer = coin(90, "pcoin");
        let expected_fees = coin(10, "pcoin");
        res.messages.into_iter().for_each(|msg| match msg {
            CosmosMsg::Bank(BankMsg::Send {
                amount, to_address, ..
            }) => {
                assert_eq!(amount.len(), 1);
                if to_address == HumanAddr::from("merchant") {
                    assert_eq!(amount[0], expected_transfer)
                } else if to_address == HumanAddr::from("feebucket") {
                    assert_eq!(amount[0], expected_fees)
                } else {
                    panic!("unexpected to_address in bank message")
                }
            }
            _ => panic!("unexpected message type"),
        });

        // Ensure we got the purchase ID event attribute value
        let expected_purchase_id = "a7918172-ac09-43f6-bc4b-7ac2fbad17e9";
        res.attributes.into_iter().for_each(|atr| {
            if atr.key == "purchase_id" {
                assert_eq!(atr.value, expected_purchase_id)
            }
        })
    }

    #[test]
    fn handle_invalid_funds() {
        // Init contract state
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("feebucket", &[]);
        let msg = InitMsg {
            contract_name: "tutorial.sc.pb".into(),
            purchase_denom: "pcoin".into(),
            merchant_address: HumanAddr::from("merchant"),
            fee_percent: Decimal::percent(10),
        };
        let _ = init(deps.as_mut(), env, info, msg).unwrap();

        // Test purchase message
        let msg = HandleMsg::Purchase {
            id: "a7918172-ac09-43f6-bc4b-7ac2fbad17e9".into(),
        };

        // Don't send any funds
        let info = mock_info("consumer", &[]);
        let err = handle(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        match err {
            ContractError::Std(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "no purchase funds sent")
            }
            _ => panic!("unexpected handle error"),
        }

        // Send zero amount for a valid denom
        let info = mock_info("consumer", &[coin(0, "pcoin")]);
        let err = handle(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        match err {
            ContractError::Std(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "invalid purchase funds: 0pcoin")
            }
            _ => panic!("unexpected handle error"),
        }

        // Send invalid denom
        let info = mock_info("consumer", &[coin(100, "fakecoin")]);
        let err = handle(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        match err {
            ContractError::Std(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "invalid purchase funds: 100fakecoin")
            }
            _ => panic!("unexpected handle error"),
        }
    }

    #[test]
    fn valid_migrate() {
        // Init contract state
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("feebucket", &[]);
        let msg = InitMsg {
            contract_name: "tutorial.sc.pb".into(),
            purchase_denom: "pcoin".into(),
            merchant_address: HumanAddr::from("merchant"),
            fee_percent: Decimal::percent(5),
        };
        let _ = init(deps.as_mut(), env, info, msg).unwrap(); // Panics on error

        // Migrate with the correct account and fee percent within valid range
        let msg = MigrateMsg {
            new_fee_percent: Decimal::percent(10),
        };
        let env = mock_env();
        let info = mock_info("feebucket", &[]);
        let _ = migrate(deps.as_mut(), env, info, msg).unwrap(); // Panics on error

        // Query and check fee percentage was updated
        let msg = QueryMsg::QueryRequest {};
        let bin = query(deps.as_ref(), mock_env(), msg).unwrap();
        let resp: QueryResponse = from_binary(&bin).unwrap();
        assert_eq!(resp.fee_percent, Decimal::percent(10))
    }

    #[test]
    fn invalid_migrate() {
        // Init contract state
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();
        let info = mock_info("feebucket", &[]);
        let msg = InitMsg {
            contract_name: "tutorial.sc.pb".into(),
            purchase_denom: "pcoin".into(),
            merchant_address: HumanAddr::from("merchant"),
            fee_percent: Decimal::percent(5),
        };
        let _ = init(deps.as_mut(), env, info, msg).unwrap(); // Panics on error

        // Migrate with an invalid fee
        let msg = MigrateMsg {
            new_fee_percent: Decimal::percent(37), // error
        };
        let env = mock_env();
        let info = mock_info("feebucket", &[]);
        let err = migrate(deps.as_mut(), env, info, msg).unwrap_err();
        match err {
            ContractError::Std(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "fee percent must be > 0.0 and <= 0.15")
            }
            _ => panic!("unexpected init error"),
        }

        // Migrate with the incorrect account
        let msg = MigrateMsg {
            new_fee_percent: Decimal::percent(10),
        };
        let env = mock_env();
        let info = mock_info("merchant", &[]); // error
        let err = migrate(deps.as_mut(), env, info, msg).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!("unexpected init error"),
        }
    }
}
