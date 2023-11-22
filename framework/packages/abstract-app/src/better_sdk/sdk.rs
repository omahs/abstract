use crate::{better_sdk::contexts::AppExecCtx, state::AppState, AppError};
use abstract_core::{
    app::{AppConfigResponse, BaseInstantiateMsg, BaseMigrateMsg},
    module_factory::{ContextResponse, QueryMsg as FactoryQuery},
    objects::{
        dependency::{Dependency, StaticDependency},
        module::ModuleId,
        module_version::{assert_contract_upgrade, set_module_data, ModuleDataResponse, MODULE},
    },
    IBC_CLIENT,
};
use abstract_sdk::{
    base::VersionString,
    cw_helpers::wasm_smart_query,
    feature_objects::{AnsHost, VersionControlContract},
    namespaces::{ADMIN_NAMESPACE, BASE_STATE_NAMESPACE},
    AbstractSdkError,
};
use cosmwasm_std::{Response, StdError};
use cw2::set_contract_version;
use cw_controllers::{Admin, AdminResponse};
use cw_storage_plus::Item;

use super::{
    contexts::{AppInstantiateCtx, AppMigrateCtx, AppQueryCtx},
    execution_stack::DepsAccess,
    modules::ModuleInterface,
};

pub trait SylviaAbstractContract {
    type BaseInstantiateMsg: 'static;
    type BaseMigrateMsg: 'static;
}

pub const ADMIN: Admin = Admin::new(ADMIN_NAMESPACE);
pub const BASE_STATE: Item<'static, AppState> = Item::new(BASE_STATE_NAMESPACE);
// This storage is supposed to be immutable
pub const DEPENDENCIES_NAMESPACE: &str = "abstract_dependencies";
pub const DEPENDENCIES: Item<'static, Vec<Dependency>> = Item::new(DEPENDENCIES_NAMESPACE);

pub struct ModuleStateInfo {
    pub name: ModuleId<'static>,
    pub version: VersionString,
    pub metadata: Option<&'static str>,
}

impl ModuleStateInfo {
    pub const fn new(
        name: &'static str,
        version: &'static str,
        metadata: Option<&'static str>,
    ) -> Self {
        Self {
            name,
            version,
            metadata,
        }
    }
}

#[sylvia::interface]
pub trait AbstractAppBase {
    type Error: From<AppError> + From<StdError>;

    const INFO: ModuleStateInfo;
    const DEPENDENCIES: &'static [StaticDependency];

    fn admin(&self) -> Admin {
        ADMIN
    }

    fn base_ibc(&self, ctx: AppExecCtx) -> Result<(), AppError> {
        let ibc_client = ctx.modules().module_address(IBC_CLIENT)?;
        if ctx.info.sender.ne(&ibc_client) {
            return Err(AbstractSdkError::CallbackNotCalledByIbcClient {
                caller: ctx.info.sender,
                client_addr: ibc_client,
                module: Self::INFO.name.to_string(),
            }
            .into());
        };
        Ok(())
    }

    fn base_instantiate(
        &self,
        ctx: &mut AppInstantiateCtx,
        base_msg: BaseInstantiateMsg,
    ) -> Result<(), AppError> {
        let BaseInstantiateMsg {
            ans_host_address,
            version_control_address,
        } = base_msg;
        let ans_host = AnsHost {
            address: ctx.api().addr_validate(&ans_host_address)?,
        };
        let version_control = VersionControlContract {
            address: ctx.api().addr_validate(&version_control_address)?,
        };

        // TODO: Would be nice to remove context
        // Issue: We can't pass easily AccountBase with BaseInstantiateMsg (right now)

        let resp: ContextResponse = ctx.deps.querier.query(&wasm_smart_query(
            ctx.info.sender.to_string(),
            &FactoryQuery::Context {},
        )?)?;

        let account_base = resp.account_base;

        let ModuleStateInfo {
            name,
            version,
            metadata,
        } = Self::INFO;
        // Base state
        let state = AppState {
            proxy_address: account_base.proxy.clone(),
            ans_host,
            version_control,
        };

        set_module_data(
            ctx.deps.storage,
            name,
            version,
            Self::DEPENDENCIES,
            metadata,
        )?;
        set_contract_version(ctx.deps.storage, name, version)?;

        BASE_STATE.save(ctx.deps.storage, &state)?;
        ADMIN.set(ctx.deps_mut(), Some(account_base.manager))?;

        Ok(())
    }

    fn base_migrate(
        &self,
        ctx: &mut AppMigrateCtx,
        _base_msg: BaseMigrateMsg,
    ) -> Result<(), AppError> {
        let ModuleStateInfo {
            name,
            version: version_string,
            metadata,
        } = Self::INFO;

        let to_version = version_string.parse().unwrap();
        assert_contract_upgrade(ctx.deps.storage, name, to_version)?;
        set_module_data(
            ctx.deps.storage,
            name,
            version_string,
            Self::DEPENDENCIES,
            metadata,
        )?;
        set_contract_version(ctx.deps.storage, name, version_string)?;

        Ok(())
    }

    #[msg(exec)]
    fn update_config(
        &self,
        ctx: AppExecCtx,
        ans_host_address: Option<String>,
        version_control_address: Option<String>,
    ) -> Result<Response, AppError> {
        // self._update_config(deps, info, ans_host_address)?;
        // Only the admin should be able to call this
        ADMIN.assert_admin(ctx.deps.as_ref(), &ctx.info.sender)?;

        let mut state = BASE_STATE.load(ctx.deps.storage)?;

        if let Some(ans_host_address) = ans_host_address {
            state.ans_host.address = ctx.api().addr_validate(ans_host_address.as_str())?;
        }

        if let Some(version_control_address) = version_control_address {
            state.version_control.address =
                ctx.api().addr_validate(version_control_address.as_str())?;
        }

        BASE_STATE.save(ctx.deps.storage, &state)?;

        Ok(Response::new())
    }

    #[msg(query)]
    fn base_config(
        &self,
        ctx: AppQueryCtx,
    ) -> Result<abstract_core::app::AppConfigResponse, AppError> {
        let state = BASE_STATE.load(ctx.deps.storage)?;
        let admin = ADMIN.get(ctx.deps)?.unwrap();
        Ok(AppConfigResponse {
            proxy_address: state.proxy_address,
            ans_host_address: state.ans_host.address,
            manager_address: admin,
        })
    }

    #[msg(query)]
    fn base_admin(&self, ctx: AppQueryCtx) -> Result<AdminResponse, AppError> {
        Ok(ADMIN.query_admin(ctx.deps)?)
    }

    #[msg(query)]
    fn module_data(
        &self,
        ctx: AppQueryCtx,
    ) -> Result<abstract_core::objects::module_version::ModuleDataResponse, AppError> {
        let module_data = MODULE.load(ctx.deps.storage)?;
        Ok(ModuleDataResponse {
            module_id: module_data.module,
            version: module_data.version,
            dependencies: module_data
                .dependencies
                .into_iter()
                .map(Into::into)
                .collect(),
            metadata: module_data.metadata,
        })
    }
}
pub struct AbstractApp;
impl SylviaAbstractContract for AbstractApp {
    type BaseInstantiateMsg = abstract_core::app::BaseInstantiateMsg;
    type BaseMigrateMsg = abstract_core::app::BaseMigrateMsg;
}
