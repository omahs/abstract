use abstract_core::{
    objects::module::{ModuleInfo, ModuleVersion},
    AbstractError, IBC_HOST,
};
use abstract_sdk::{base::ModuleIbcEndpoint, features::AbstractRegistryAccess};
use cosmwasm_std::Addr;

use crate::{state::ContractError, AppContract};

impl<
        Error: ContractError,
        CustomInitMsg,
        CustomExecMsg,
        CustomQueryMsg,
        CustomMigrateMsg,
        ReceiveMsg,
        SudoMsg,
    > ModuleIbcEndpoint
    for AppContract<
        Error,
        CustomInitMsg,
        CustomExecMsg,
        CustomQueryMsg,
        CustomMigrateMsg,
        ReceiveMsg,
        SudoMsg,
    >
{
    fn ibc_host(&self, deps: cosmwasm_std::Deps) -> Result<Addr, Self::Error> {
        let vc_query_result = self
            .abstract_registry(deps)?
            .query_module(
                ModuleInfo::from_id(
                    IBC_HOST,
                    ModuleVersion::from(abstract_ibc_host::contract::CONTRACT_VERSION),
                )?,
                &deps.querier,
            )
            .map_err(Into::<AbstractError>::into)?;

        Ok(vc_query_result.reference.unwrap_native()?)
    }
}
