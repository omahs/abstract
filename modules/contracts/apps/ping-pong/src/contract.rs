use crate::msg::AppMigrateMsg;
use crate::{
    error::AppError,
    handlers,
    msg::{AppExecuteMsg, AppInstantiateMsg, AppQueryMsg},
    replies::{self, INSTANTIATE_REPLY_ID},
};
use abstract_app::objects::dependency::StaticDependency;
use abstract_app::std::IBC_CLIENT;
use abstract_app::AppContract;

use cosmwasm_std::Response;

/// The version of your app
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The id of the app
pub const APP_ID: &str = "abstract:ping-pong";

/// The type of the result returned by your app's entry points.
pub type AppResult<T = Response> = Result<T, AppError>;

/// The type of the app that is used to build your app and access the Abstract SDK features.
pub type App = AppContract<AppError, AppInstantiateMsg, AppExecuteMsg, AppQueryMsg, AppMigrateMsg>;

const APP: App = App::new(APP_ID, APP_VERSION, None)
    .with_instantiate(handlers::instantiate_handler)
    .with_execute(handlers::execute_handler)
    .with_query(handlers::query_handler)
    .with_migrate(handlers::migrate_handler)
    .with_replies(&[(INSTANTIATE_REPLY_ID, replies::instantiate_reply)])
    .with_dependencies(&[StaticDependency::new(
        IBC_CLIENT,
        &[abstract_ibc_client::contract::CONTRACT_VERSION],
    )])
    .with_module_ibc(crate::ibc::receive_module_ibc);

// Export handlers
#[cfg(feature = "export")]
abstract_app::export_endpoints!(APP, App);

abstract_app::cw_orch_interface!(APP, App, AppInterface);

#[cfg(not(target_arch = "wasm32"))]
use abstract_app::std::manager::ModuleInstallConfig;
#[cfg(not(target_arch = "wasm32"))]
impl<Chain: cw_orch::environment::CwEnv> abstract_app::abstract_interface::DependencyCreation
    for crate::AppInterface<Chain>
{
    type DependenciesConfig = cosmwasm_std::Empty;

    fn dependency_install_configs(
        _configuration: Self::DependenciesConfig,
    ) -> Result<Vec<ModuleInstallConfig>, abstract_app::abstract_interface::AbstractInterfaceError>
    {
        Ok(vec![ModuleInstallConfig::new(
            abstract_app::objects::module::ModuleInfo::from_id(
                IBC_CLIENT,
                abstract_ibc_client::contract::CONTRACT_VERSION.into(),
            )?,
            None,
        )])
    }
}