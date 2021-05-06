use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("InvalidPrice: {message:?}")]
    InvalidPrice { message: String },

    #[error("DuplicateBid: {id:?}")]
    DuplicateBid { id: String },

    #[error("DuplicateAsk: {id:?}")]
    DuplicateAsk { id: String },

    #[error("InvalidFunds: {message:?}")]
    InvalidFunds { message: String },

    #[error("AskClosed")]
    AskClosed {},

    #[error("BidClosed")]
    BidClosed {},
}
