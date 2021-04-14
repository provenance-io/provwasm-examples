use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("LoanCapExceeded: amount={amount:?} loans={loans:?} loan_cap={loan_cap:?}")]
    LoanCapExceeded {
        amount: Uint128,
        loans: Uint128,
        loan_cap: Uint128,
    },
    #[error("InsufficientFunds")]
    InsufficientFunds {},
    #[error("InvalidBuy")]
    InvalidBuy {},
    #[error("InvalidSell")]
    InvalidSell {},
    #[error("InvalidFundsDenom")]
    InvalidFundsDenom {},
    #[error("UnknownTrader")]
    UnknownTrader {},
}
