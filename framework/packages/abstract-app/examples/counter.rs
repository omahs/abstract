pub use abstract_core::app;

pub use cosmwasm_std::testing::*;
use cosmwasm_std::{Response, StdError};

pub type CounterResult<T = Response> = Result<T, CounterError>;

#[cosmwasm_schema::cw_serde]
pub struct CounterInitMsg;

#[cosmwasm_schema::cw_serde]
pub enum CounterExecMsg {
    UpdateConfig {},
}

#[cosmwasm_schema::cw_serde]
pub struct CounterQueryMsg;

#[cosmwasm_schema::cw_serde]
pub struct CounterMigrateMsg;

#[cosmwasm_schema::cw_serde]
pub struct CounterReceiveMsg;

#[cosmwasm_schema::cw_serde]
pub struct CounterSudoMsg;

abstract_app::app_msg_types!(CounterApp, CounterExecMsg, CounterQueryMsg);

use abstract_app::{AppContract, AppError};

use abstract_sdk::{features::DepsType, AbstractSdkError};

use cw_controllers::AdminError;
use thiserror::Error;

// ANCHOR: error
#[derive(Error, Debug, PartialEq)]
pub enum CounterError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    DappError(#[from] AppError),

    #[error("{0}")]
    Abstract(#[from] abstract_core::AbstractError),

    #[error("{0}")]
    AbstractSdk(#[from] AbstractSdkError),

    #[error("{0}")]
    Unauthorized(#[from] AdminError),
}
// ANCHOR_END: error

// ANCHOR: counter_app
pub type CounterApp<'a> = AppContract<
    'a,
    CounterError,
    CounterInitMsg,
    CounterExecMsg,
    CounterQueryMsg,
    CounterMigrateMsg,
    CounterReceiveMsg,
    CounterSudoMsg,
>;
// ANCHOR_END: counter_app

const COUNTER_ID: &str = "example:counter";
const APP_VERSION: &str = "1.0.0";

// ANCHOR: handlers
// ANCHOR: new

pub fn counter(deps: DepsType) -> CounterApp {
    CounterApp::new(deps, COUNTER_ID, APP_VERSION, None)
        // ANCHOR_END: new
        .with_instantiate(handlers::instantiate)
        .with_execute(handlers::execute)
        .with_query(handlers::query)
        .with_sudo(handlers::sudo)
        .with_receive(handlers::receive)
        .with_replies(&[(1u64, handlers::reply)])
        .with_migrate(handlers::migrate)
}
// ANCHOR_END: handlers

// ANCHOR: export
abstract_app::export_endpoints!(counter, CounterApp);
// ANCHOR_END: export

// ANCHOR: interface
abstract_app::cw_orch_interface!(COUNTER_APP, CounterApp, CounterAppInterface);

// Testing here to see if something is possible with lifetimes

// This struct represents the interface to the contract.
pub struct CounterAppInterface<'a, Chain: ::cw_orch::prelude::CwEnv>(
    ::cw_orch::contract::Contract<Chain>,
    ::std::marker::PhantomData<&'a ()>,
);

impl<'a, Chain: ::cw_orch::prelude::CwEnv> CounterAppInterface<'a, Chain> {
    /// Constructor for the contract interface
    pub fn new(contract_id: impl ToString, chain: Chain) -> Self {
        Self(
            ::cw_orch::contract::Contract::new(contract_id, chain),
            ::std::marker::PhantomData,
        )
    }
}

// Traits for signaling cw-orchestrator with what messages to call the contract's entry points.
impl<'a, Chain: ::cw_orch::prelude::CwEnv> ::cw_orch::prelude::InstantiableContract
    for CounterAppInterface<'a, Chain>
{
    type InstantiateMsg = InstantiateMsg<'a>;
}
impl<'a, Chain: ::cw_orch::prelude::CwEnv> ::cw_orch::prelude::ExecutableContract
    for CounterAppInterface<'a, Chain>
{
    type ExecuteMsg = ExecuteMsg<'a>;
}

// ANCHOR_END: interface

mod handlers {
    #![allow(non_upper_case_globals)]
    use abstract_sdk::{
        features::{CustomData, DepsAccess},
        AbstractResponse,
    };
    use cosmwasm_std::*;

    use super::*;

    pub fn instantiate(app: &mut CounterApp, _msg: CounterInitMsg) -> CounterResult<()> {
        app.set_data("counter_init".as_bytes());
        Ok(())
    }
    pub fn query(_app: &CounterApp, _msg: CounterQueryMsg) -> CounterResult<Binary> {
        to_json_binary("counter_query").map_err(Into::into)
    }
    pub fn sudo(app: &mut CounterApp, _msg: CounterSudoMsg) -> CounterResult<()> {
        app.set_data("counter_sudo".as_bytes());
        Ok(())
    }
    pub fn receive(app: &mut CounterApp, _msg: CounterReceiveMsg) -> CounterResult<()> {
        app.set_data("counter_receive".as_bytes());
        Ok(())
    }
    pub fn reply(app: &mut CounterApp, msg: Reply) -> CounterResult<()> {
        app.set_data(msg.result.unwrap().data.unwrap());
        Ok(())
    }
    pub fn migrate(app: &mut CounterApp, _msg: CounterMigrateMsg) -> CounterResult<()> {
        app.set_data("counter_migrate".as_bytes());
        Ok(())
    }
    // ANCHOR: execute
    pub fn execute(
        app: &mut CounterApp, // <-- Notice how the `CounterApp` is available here
        msg: CounterExecMsg,
    ) -> CounterResult<()> {
        match msg {
            CounterExecMsg::UpdateConfig {} => update_config(app),
        }
    }

    /// Update the configuration of the app
    fn update_config(app: &mut CounterApp) -> CounterResult<()> {
        // Only the admin should be able to call this
        app.admin
            .assert_admin(app.deps.deps(), &app.message_info().sender)?;

        app.tag_response("update_config");
        app.set_data("counter_exec".as_bytes());
        Ok(())
    }
    // ANCHOR_END: execute
}

fn main() {}
