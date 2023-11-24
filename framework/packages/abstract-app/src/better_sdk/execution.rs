//! # Executor
//! The executor provides function for executing commands on the Account.
//!

use abstract_sdk::{AbstractSdkResult, AccountAction};
use cosmwasm_std::{CosmosMsg, ReplyOn};

use super::execution_stack::{Executable, ExecutionStack};

/// Execute an `AccountAction` on the Account.
pub trait Execution: ExecutionStack {
    /**
        API for executing [`AccountAction`]s on the Account.
        Group your actions together in a single execute call if possible.

        Executing [`CosmosMsg`] on the account is possible by creating an [`AccountAction`].

        # Example
        ```
        use abstract_sdk::prelude::*;
        # use cosmwasm_std::testing::mock_dependencies;
        # use abstract_sdk::mock_module::MockModule;
        # let module = MockModule::new();
        # let deps = mock_dependencies();

        let executor: Executor<MockModule>  = module.executor(deps.as_ref());
        ```
    */
    fn executor(&mut self) -> Executor<Self> {
        Executor { base: self }
    }
}

impl<T> Execution for T where T: ExecutionStack {}

/**
    API for executing [`AccountAction`]s on the Account.
    Group your actions together in a single execute call if possible.

    Executing [`CosmosMsg`] on the account is possible by creating an [`AccountAction`].

    # Example
    ```
    use abstract_sdk::prelude::*;
    # use cosmwasm_std::testing::mock_dependencies;
    # use abstract_sdk::mock_module::MockModule;
    # let module = MockModule::new();
    # let deps = mock_dependencies();

    let executor: Executor<MockModule>  = module.executor(deps.as_ref());
    ```
*/
pub struct Executor<'a, T: Execution> {
    base: &'a mut T,
}

impl<'a, T: Execution> Executor<'a, T> {
    /// Execute the msgs on the Account.
    /// These messages will be executed on the proxy contract and the sending module must be whitelisted.
    pub fn execute(&mut self, actions: Vec<CosmosMsg>) -> AbstractSdkResult<()> {
        self.base
            .push_executable(Executable::AccountAction(AccountAction::from_vec(actions)));
        Ok(())
    }

    /// Execute the msgs on the Account.
    /// These messages will be executed on the proxy contract and the sending module must be whitelisted.
    /// The execution will be executed in a submessage and the reply will be sent to the provided `reply_on`.
    pub fn execute_with_reply(
        &mut self,
        msgs: Vec<CosmosMsg>,
        reply_on: ReplyOn,
        id: u64,
    ) -> AbstractSdkResult<()> {
        self.base
            .push_executable(Executable::SubMsg { msgs, reply_on, id });
        Ok(())
    }

    pub fn execute_with_options(
        &mut self,
        msgs: Vec<CosmosMsg>,
        options: &Option<ExecutorOptions>,
    ) -> AbstractSdkResult<()> {
        match options {
            Some(ExecutorOptions { reply_on, id }) => {
                self.execute_with_reply(msgs, reply_on.clone(), *id)
            }
            None => self.execute(msgs),
        }
    }
}

pub struct ExecutorOptions {
    pub reply_on: ReplyOn,
    pub id: u64,
}

/// CosmosMsg from the executor methods
#[must_use = "ExecutorMsg should be provided to Response::add_message"]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Eq))]
pub struct ExecutorMsg(CosmosMsg);

impl From<ExecutorMsg> for CosmosMsg {
    fn from(val: ExecutorMsg) -> Self {
        val.0
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use abstract_core::proxy::ExecuteMsg;
//     use abstract_testing::prelude::*;
//     use cosmwasm_std::{testing::*, *};
//     use speculoos::prelude::*;

//     fn mock_bank_send(amount: Vec<Coin>) -> AccountAction {
//         AccountAction::from(CosmosMsg::Bank(BankMsg::Send {
//             to_address: "to_address".to_string(),
//             amount,
//         }))
//     }

//     fn flatten_actions(actions: Vec<AccountAction>) -> Vec<CosmosMsg> {
//         actions.into_iter().flat_map(|a| a.messages()).collect()
//     }

//     mod execute {
//         use super::*;
//         use cosmwasm_std::to_json_binary;

//         /// Tests that no error is thrown with empty messages provided
//         #[test]
//         fn empty_actions() {
//             let deps = mock_dependencies();
//             let stub = MockModule::new();
//             let executor = stub.executor(deps.as_ref());

//             let messages = vec![];

//             let actual_res = executor.execute(messages.clone());
//             assert_that!(actual_res).is_ok();

//             let expected = ExecutorMsg(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: TEST_PROXY.to_string(),
//                 msg: to_json_binary(&ExecuteMsg::ModuleAction {
//                     msgs: flatten_actions(messages),
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             }));
//             assert_that!(actual_res.unwrap()).is_equal_to(expected);
//         }

//         #[test]
//         fn with_actions() {
//             let deps = mock_dependencies();
//             let stub = MockModule::new();
//             let executor = stub.executor(deps.as_ref());

//             // build a bank message
//             let messages = vec![mock_bank_send(coins(100, "juno"))];

//             let actual_res = executor.execute(messages.clone());
//             assert_that!(actual_res).is_ok();

//             let expected = ExecutorMsg(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: TEST_PROXY.to_string(),
//                 msg: to_json_binary(&ExecuteMsg::ModuleAction {
//                     msgs: flatten_actions(messages),
//                 })
//                 .unwrap(),
//                 // funds should be empty
//                 funds: vec![],
//             }));
//             assert_that!(actual_res.unwrap()).is_equal_to(expected);
//         }
//     }

//     mod execute_with_reply {
//         use super::*;

//         /// Tests that no error is thrown with empty messages provided
//         #[test]
//         fn empty_actions() {
//             let deps = mock_dependencies();
//             let stub = MockModule::new();
//             let executor = stub.executor(deps.as_ref());

//             let empty_actions = vec![];
//             let expected_reply_on = ReplyOn::Success;
//             let expected_reply_id = 10952;

//             let actual_res = executor.execute_with_reply(
//                 empty_actions.clone(),
//                 expected_reply_on.clone(),
//                 expected_reply_id,
//             );
//             assert_that!(actual_res).is_ok();

//             let expected = SubMsg {
//                 id: expected_reply_id,
//                 msg: CosmosMsg::Wasm(WasmMsg::Execute {
//                     contract_addr: TEST_PROXY.to_string(),
//                     msg: to_json_binary(&ExecuteMsg::ModuleAction {
//                         msgs: flatten_actions(empty_actions),
//                     })
//                     .unwrap(),
//                     funds: vec![],
//                 }),
//                 gas_limit: None,
//                 reply_on: expected_reply_on,
//             };
//             assert_that!(actual_res.unwrap()).is_equal_to(expected);
//         }

//         #[test]
//         fn with_actions() {
//             let deps = mock_dependencies();
//             let stub = MockModule::new();
//             let executor = stub.executor(deps.as_ref());

//             // build a bank message
//             let action = vec![mock_bank_send(coins(1, "denom"))];
//             // reply on never
//             let expected_reply_on = ReplyOn::Never;
//             let expected_reply_id = 1;

//             let actual_res = executor.execute_with_reply(
//                 action.clone(),
//                 expected_reply_on.clone(),
//                 expected_reply_id,
//             );
//             assert_that!(actual_res).is_ok();

//             let expected = SubMsg {
//                 id: expected_reply_id,
//                 msg: CosmosMsg::Wasm(WasmMsg::Execute {
//                     contract_addr: TEST_PROXY.to_string(),
//                     msg: to_json_binary(&ExecuteMsg::ModuleAction {
//                         msgs: flatten_actions(action),
//                     })
//                     .unwrap(),
//                     // funds should be empty
//                     funds: vec![],
//                 }),
//                 gas_limit: None,
//                 reply_on: expected_reply_on,
//             };
//             assert_that!(actual_res.unwrap()).is_equal_to(expected);
//         }
//     }

//     mod execute_with_response {
//         use super::*;
//         use cosmwasm_std::coins;

//         /// Tests that no error is thrown with empty messages provided
//         #[test]
//         fn empty_actions() {
//             let deps = mock_dependencies();
//             let stub = MockModule::new();
//             let executor = stub.executor(deps.as_ref());

//             let empty_actions = vec![];
//             let expected_action = "THIS IS AN ACTION";

//             let actual_res = executor.execute_with_response(empty_actions.clone(), expected_action);

//             let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: TEST_PROXY.to_string(),
//                 msg: to_json_binary(&ExecuteMsg::ModuleAction {
//                     msgs: flatten_actions(empty_actions),
//                 })
//                 .unwrap(),
//                 funds: vec![],
//             });

//             let expected = Response::new()
//                 .add_event(
//                     Event::new("abstract")
//                         .add_attribute("contract", stub.module_id())
//                         .add_attribute("action", expected_action),
//                 )
//                 .add_message(expected_msg);

//             assert_that!(actual_res).is_ok().is_equal_to(expected);
//         }

//         #[test]
//         fn with_actions() {
//             let deps = mock_dependencies();
//             let stub = MockModule::new();
//             let executor = stub.executor(deps.as_ref());

//             // build a bank message
//             let action = vec![mock_bank_send(coins(1, "denom"))];
//             let expected_action = "provide liquidity";

//             let actual_res = executor.execute_with_response(action.clone(), expected_action);

//             let expected_msg = CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: TEST_PROXY.to_string(),
//                 msg: to_json_binary(&ExecuteMsg::ModuleAction {
//                     msgs: flatten_actions(action),
//                 })
//                 .unwrap(),
//                 // funds should be empty
//                 funds: vec![],
//             });
//             let expected = Response::new()
//                 .add_event(
//                     Event::new("abstract")
//                         .add_attribute("contract", stub.module_id())
//                         .add_attribute("action", expected_action),
//                 )
//                 .add_message(expected_msg);
//             assert_that!(actual_res).is_ok().is_equal_to(expected);
//         }
//     }
// }