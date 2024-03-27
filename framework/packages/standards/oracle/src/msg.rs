#![warn(missing_docs)]
//! # Oracle Adapter API
// re-export response types
use abstract_core::{
    adapter,
    objects::{
        price_source::{PriceSource, UncheckedPriceSource},
        AssetEntry,
    },
};
use cosmwasm_schema::QueryResponses;
use cosmwasm_std::Uint128;
use cw_asset::AssetInfo;

pub use crate::action::OracleConfiguration;
use crate::state::{AccountValue, Complexity};

/// The name of the oracle to trade on.
pub type ProviderName = String;

/// Top-level Abstract Adapter execute message. This is the message that is passed to the `execute` entrypoint of the smart-contract.
pub type ExecuteMsg = adapter::ExecuteMsg<OracleExecuteMsg>;
/// Top-level Abstract Adapter instantiate message. This is the message that is passed to the `instantiate` entrypoint of the smart-contract.
pub type InstantiateMsg = adapter::InstantiateMsg<OracleInstantiateMsg>;
/// Top-level Abstract Adapter query message. This is the message that is passed to the `query` entrypoint of the smart-contract.
pub type QueryMsg = adapter::QueryMsg<OracleQueryMsg>;

impl adapter::AdapterExecuteMsg for OracleExecuteMsg {}
impl adapter::AdapterQueryMsg for OracleQueryMsg {}

/// Instantiation message for oracle adapter
#[cosmwasm_schema::cw_serde]
pub struct OracleInstantiateMsg {
    /// Maximum age of external price source before getting filtrated in calculations
    external_age_max: u64,
}

/// Oracle Execute msg
#[cosmwasm_schema::cw_serde]
pub enum OracleExecuteMsg {
    /// Admin configuration to perform on the Oracle adapter
    /// This can be done only oracle admin and will be used for default values during queries
    AdminAction(OracleConfiguration),
    /// Configuration to perform on the Oracle adapter
    AccountAction(OracleConfiguration),
}

/// Query messages for the oracle adapter
#[cosmwasm_schema::cw_serde]
pub enum OracleQueryMsg {
    /// Query for arbitrary address
    /// Default values provided by adapter owner used for calculations
    Address {
        address: String,
        query_msg: AccountQueryMsg,
    },
    /// Query for proxy address
    Account {
        proxy_address: String,
        query_msg: AccountQueryMsg,
    },
}

#[cosmwasm_schema::cw_serde]
#[derive(QueryResponses)]
pub enum AddressQueryMsg {
    /// Default oracle adapter configuration
    #[returns(AccountConfig)]
    Config {},
}

#[cosmwasm_schema::cw_serde]
#[derive(QueryResponses)]
pub enum AccountQueryMsg {
    /// Account oracle adapter configuration
    #[returns(AccountConfig)]
    Config {},
    /// Returns the total value of the assets held by this account
    #[returns(AccountValue)]
    TotalValue {},
    /// Returns the value of given tokens on account
    #[returns(TokensValueResponse)]
    TokensValue { identifiers: Vec<AssetEntry> },
    /// Returns the amounts of specified tokens this account holds
    #[returns(HoldingAmountsResponse)]
    HoldingAmounts { identifiers: Vec<AssetEntry> },
    /// Returns price sources for given asset entries
    #[returns(AssetPriceSourcesResponse)]
    AssetPriceSources { identifier: Vec<AssetEntry> },
    /// Returns list of identifiers
    #[returns(AssetIdentifiersResponse)]
    AssetIdentifiers {
        start_after: Option<AssetEntry>,
        limit: Option<u8>,
    },
    /// Returns base asset
    #[returns(BaseAssetResponse)]
    BaseAsset {},
}

/// Account oracle configuration
#[cosmwasm_schema::cw_serde]
pub struct AccountConfig {
    external_age_max: u64,
}

#[cosmwasm_schema::cw_serde]
pub struct OracleAsset {
    pub price_source: PriceSource,
    pub complexity: Complexity,
}

#[cosmwasm_schema::cw_serde]
pub struct TokensValueResponse {
    pub tokens_value: Vec<(AssetEntry, Uint128)>,
}

#[cosmwasm_schema::cw_serde]
pub struct HoldingAmountsResponse {
    pub amounts: Vec<(AssetEntry, Uint128)>,
}

#[cosmwasm_schema::cw_serde]
pub struct BaseAssetResponse {
    pub base_asset: AssetInfo,
}

/// Human readable config for a single asset
#[cosmwasm_schema::cw_serde]
pub struct AssetPriceSourcesResponse {
    pub sources: Vec<(AssetEntry, UncheckedPriceSource)>,
}

#[cosmwasm_schema::cw_serde]
pub struct AssetIdentifiersResponse {
    pub identifiers: Vec<AssetEntry>,
}
