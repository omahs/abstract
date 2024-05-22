use abstract_app::objects::chain_name::ChainName;
use cosmwasm_schema::QueryResponses;

use crate::contract::App;

// This is used for type safety and re-exporting the contract endpoint structs.
abstract_app::app_msg_types!(App, AppExecuteMsg, AppQueryMsg);

/// App instantiate message
#[cosmwasm_schema::cw_serde]
pub struct AppInstantiateMsg {}

/// App execute messages
#[cosmwasm_schema::cw_serde]
#[derive(cw_orch::ExecuteFns)]
#[impl_into(ExecuteMsg)]
pub enum AppExecuteMsg {
    /// PingPong between this module on other chain
    PingPong {
        /// How many pings pongs in and out should be done
        pongs: u32,
        /// Host chain
        host_chain: ChainName,
    },
}

/// App query messages
#[cosmwasm_schema::cw_serde]
#[derive(QueryResponses, cw_orch::QueryFns)]
#[impl_into(QueryMsg)]
pub enum AppQueryMsg {
    #[returns(PongsResponse)]
    Pongs {},
    /// Returns last ping pong that was initiated through this smart contract
    #[returns(PreviousPingPongResponse)]
    PreviousPingPong {},
    // TODO: Rematch{} Ibc query remote chain and if last match was us we pingpong again
}

#[cosmwasm_schema::cw_serde]
pub struct PingPongIbcMsg {
    pub pongs: u32,
}

#[cosmwasm_schema::cw_serde]
pub struct AppMigrateMsg {}

#[cosmwasm_schema::cw_serde]
pub struct PongsResponse {
    pub pongs: u32,
}

#[cosmwasm_schema::cw_serde]
pub struct PreviousPingPongResponse {
    pub pongs: Option<u32>,
    pub host_chain: Option<ChainName>,
}
