use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("LoanCapExceeded")]
    LoanCapExceeded {},
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
