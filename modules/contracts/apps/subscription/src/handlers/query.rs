use cosmwasm_std::{to_binary, Binary, Deps, Env, StdError, Uint128};
use cw_asset::Asset;

use crate::{
    contract::{SubscriptionApp, SubscriptionResult, BLOCKS_PER_MONTH},
    msg::{
        ConfigResponse, StateResponse, SubscriberStateResponse, SubscriptionFeeResponse,
        SubscriptionQueryMsg,
    },
    state::{DORMANT_SUBSCRIBERS, SUBSCRIBERS, SUBSCRIPTION_CONFIG, SUBSCRIPTION_STATE},
};

pub fn query_handler(
    deps: Deps,
    _env: Env,
    app: &SubscriptionApp,
    msg: SubscriptionQueryMsg,
) -> SubscriptionResult<Binary> {
    match msg {
        // handle dapp-specific queries here
        SubscriptionQueryMsg::State {} => {
            let subscription_state = SUBSCRIPTION_STATE.load(deps.storage)?;
            // TODO: let contributor_state = CONTRIBUTION_STATE.load(deps.storage)?;
            to_binary(&StateResponse {
                // contribution: contributor_state,
                subscription: subscription_state,
            })
        }
        SubscriptionQueryMsg::Fee {} => {
            let config = SUBSCRIPTION_CONFIG.load(deps.storage)?;
            let minimal_cost = Uint128::from(BLOCKS_PER_MONTH) * config.subscription_cost_per_block;
            to_binary(&SubscriptionFeeResponse {
                fee: Asset {
                    info: config.payment_asset,
                    amount: minimal_cost,
                },
            })
        }
        SubscriptionQueryMsg::Config {} => {
            let subscription_config = SUBSCRIPTION_CONFIG.load(deps.storage)?;
            // TODO: let contributor_config = CONTRIBUTION_CONFIG.load(deps.storage)?;
            to_binary(&ConfigResponse {
                // contribution: contributor_config,
                subscription: subscription_config,
            })
        }
        SubscriptionQueryMsg::SubscriberState { os_id } => {
            let maybe_sub = SUBSCRIBERS.may_load(deps.storage, &os_id)?;
            let maybe_dormant_sub = DORMANT_SUBSCRIBERS.may_load(deps.storage, &os_id)?;
            let subscription_state = if let Some(sub) = maybe_sub {
                to_binary(&SubscriberStateResponse {
                    currently_subscribed: true,
                    subscriber_details: sub,
                })?
            } else if let Some(sub) = maybe_dormant_sub {
                to_binary(&SubscriberStateResponse {
                    currently_subscribed: true,
                    subscriber_details: sub,
                })?
            } else {
                return Err(StdError::generic_err("os has os_id 0 or does not exist").into());
            };
            Ok(subscription_state)
        } // TODO:
          // SubscriptionQueryMsg::ContributorState { os_id } => {
          //     let account_registry = app.account_registry(deps);
          //     let contributor_addr = account_registry.account_base(&os_id)?.manager;
          //     let maybe_contributor = CONTRIBUTORS.may_load(deps.storage, &contributor_addr)?;
          //     let subscription_state = if let Some(compensation) = maybe_contributor {
          //         to_binary(&ContributorStateResponse { compensation })?
          //     } else {
          //         return Err(StdError::generic_err("provided address is not a contributor").into());
          //     };
          //     Ok(subscription_state)
          // }
    }
    .map_err(Into::into)
}
