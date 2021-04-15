use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("DuplicateBuy: {id:?}")]
    DuplicateBuy { id: String },

    #[error("DuplicateSell: {id:?}")]
    DuplicateSell { id: String },

    #[error("InvalidFunds: {message:?}")]
    InvalidFunds { message: String },
}
