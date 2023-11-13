use crate::adapter::CwStakingAdapter;
use crate::contract::{CwStakingAdapter as CwStakingContract, StakingResult};
use crate::resolver::{self, is_over_ibc};
use crate::CW_STAKING_ADAPTER_ID;
use abstract_core::ibc::CallbackInfo;
use abstract_core::objects::chain_name::ChainName;
use abstract_sdk::feature_objects::AnsHost;
use abstract_sdk::features::{AbstractNameService, AbstractResponse};
use abstract_sdk::{IbcInterface, Resolve};
use abstract_staking_standard::msg::{
    ExecuteMsg, ProviderName, StakingAction, StakingExecuteMsg, IBC_STAKING_PROVIDER_ID,
};
use cosmwasm_std::{to_json_binary, Coin, Deps, DepsMut, Env, MessageInfo, Response};

/// Execute staking operation locally or over IBC
pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    adapter: CwStakingContract,
    msg: StakingExecuteMsg,
) -> StakingResult {
    let StakingExecuteMsg {
        provider: provider_name,
        action,
    } = msg;
    // if provider is on an app-chain, execute the action on the app-chain
    let (local_provider_name, is_over_ibc) = is_over_ibc(env.clone(), &provider_name)?;
    if is_over_ibc {
        handle_ibc_request(&deps, info, &adapter, local_provider_name, &action)
    } else {
        // the action can be executed on the local chain
        handle_local_request(deps, env, info, adapter, action, local_provider_name)
    }
}

/// Handle an adapter request that can be executed on the local chain
fn handle_local_request(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    adapter: CwStakingContract,
    action: StakingAction,
    provider_name: String,
) -> StakingResult {
    let provider = resolver::resolve_local_provider(&provider_name)?;
    let response = Response::new()
        .add_submessage(adapter.resolve_staking_action(deps, env, info, action, provider)?);
    Ok(adapter.custom_tag_response(
        response,
        "handle_local_request",
        vec![("provider", provider_name)],
    ))
}

/// Handle a request that needs to be executed on a remote chain
/// TODO, this doesn't work as is. This should be corrected when working with ibc hooks ?
fn handle_ibc_request(
    deps: &DepsMut,
    info: MessageInfo,
    adapter: &CwStakingContract,
    provider_name: ProviderName,
    action: &StakingAction,
) -> StakingResult {
    let host_chain = ChainName::from_string(provider_name.clone())?; // TODO : Especially this line is faulty
    let ans = adapter.name_service(deps.as_ref());
    let ibc_client = adapter.ibc_client(deps.as_ref());
    // get the to-be-sent assets from the action
    let coins = resolve_assets_to_transfer(deps.as_ref(), action, ans.host())?;
    // construct the ics20 call(s)
    let ics20_transfer_msg = ibc_client.ics20_transfer(host_chain.to_string(), coins)?;
    // construct the action to be called on the host
    // construct the action to be called on the host
    let host_action = abstract_sdk::core::ibc_host::HostAction::Dispatch {
        manager_msg: abstract_core::manager::ExecuteMsg::ExecOnModule {
            module_id: CW_STAKING_ADAPTER_ID.to_string(),
            exec_msg: to_json_binary::<ExecuteMsg>(
                &StakingExecuteMsg {
                    provider: provider_name.clone(),
                    action: action.clone(),
                }
                .into(),
            )?,
        },
    };

    // If the calling entity is a contract, we provide a callback on successful cross-chain-staking
    let maybe_contract_info = deps.querier.query_wasm_contract_info(info.sender.clone());
    let callback = if maybe_contract_info.is_err() {
        None
    } else {
        Some(CallbackInfo {
            id: IBC_STAKING_PROVIDER_ID.into(),
            msg: Some(to_json_binary(&StakingExecuteMsg {
                provider: provider_name.clone(),
                action: action.clone(),
            })?),
            receiver: info.sender.into_string(),
        })
    };
    let ibc_action_msg = ibc_client.host_action(host_chain.to_string(), host_action, callback)?;

    // call both messages on the proxy
    let response = Response::new().add_messages(vec![ics20_transfer_msg, ibc_action_msg]);
    Ok(adapter.custom_tag_response(
        response,
        "handle_ibc_request",
        vec![("provider", provider_name)],
    ))
}

/// Resolve the assets to be transferred to the host chain for the given action
fn resolve_assets_to_transfer(
    deps: Deps,
    dex_action: &StakingAction,
    ans_host: &AnsHost,
) -> StakingResult<Vec<Coin>> {
    match dex_action {
        StakingAction::Stake { assets, .. } => {
            let resolved: Vec<Coin> = assets
                .resolve(&deps.querier, ans_host)?
                .into_iter()
                .map(Coin::try_from)
                .collect::<Result<_, cw_asset::AssetError>>()?;
            Ok(resolved)
        }
        _ => Ok(vec![]),
    }
}
