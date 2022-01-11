use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError,
    StdResult, SubMsg, WasmMsg, Uint64,
};

use crate::error::OsFactoryError;
use crate::response::MsgInstantiateContractResponse;
use crate::{msg::*, commands};
use crate::state::*;

pub type OsFactoryResult = Result<Response,OsFactoryError>;

#[cfg_attr(not(feature = "library"), entry_point)]
/// Set config, sender is Admin 
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> OsFactoryResult {
    let config = Config {
        version_control_contract: deps.api.addr_validate(&msg.version_control_contract)?,
        memory_contract: deps.api.addr_validate(&msg.memory_contract)?,
        creation_fee: msg.creation_fee,
        os_id_sequence: 0u32,
    };

    CONFIG.save(deps.storage, &config)?;
    ADMIN.set(deps, Some(info.sender))?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> OsFactoryResult {
    match msg {
        ExecuteMsg::UpdateConfig {
            admin,
            memory_contract,
            version_control_contract,
            creation_fee,
        } => commands::execute_update_config(deps, env, info, admin, memory_contract, version_control_contract, creation_fee),
        ExecuteMsg::CreateOs {
            governance
        } => commands::execute_create_os(deps, env, governance),
    }
}



/// This just stores the result for future query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> OsFactoryResult {

    match msg {
        Reply { id: commands::MANAGER_CREATE_ID, result } => { 
            return commands::after_manager_create_treasury(deps, result)
         },
        Reply { id: commands::TREASURY_CREATE_ID, result } => {
            return commands::after_treasury_add_to_manager(env,result)
        },
        _ => Err(OsFactoryError::UnexpectedReply{})
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state: Config = CONFIG.load(deps.storage)?;
    let admin = ADMIN.get(deps)?.unwrap();
    let resp = ConfigResponse {
        owner: admin.into(),
        version_control_contract: state.version_control_contract.into(),
        memory_contract: state.memory_contract.into(),
        creation_fee: state.creation_fee,
        os_id_sequence: state.os_id_sequence,
    };

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
