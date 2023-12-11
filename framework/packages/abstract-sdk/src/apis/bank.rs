//! # Bank
//! The Bank object handles asset transfers to and from the Account.

use crate::account_action::{ExecuteOptions, ReplyOptions};
use crate::core::objects::{AnsAsset, AssetEntry};
use crate::features::AccountIdentification;
use crate::Execution;
use crate::{ans_resolve::Resolve, features::AbstractNameService, AbstractSdkResult};
use cosmwasm_std::{to_json_binary, ReplyOn};
use cosmwasm_std::{Addr, Coin, Deps, Env};
use cw_asset::Asset;
use serde::Serialize;

/// Query and Transfer assets from and to the Abstract Account.
pub trait TransferInterface: AbstractNameService + AccountIdentification + Execution {
    /**
        API for transferring funds to and from the account.

        # Example
        ```
        use abstract_sdk::prelude::*;
        # use cosmwasm_std::testing::mock_dependencies;
        # use abstract_sdk::mock_module::MockModule;
        # let module = MockModule::new();
        # let deps = mock_dependencies();

        let bank: Bank<MockModule>  = module.bank(deps.as_ref());
        ```
    */
    fn bank(&mut self) -> Bank<Self> {
        Bank {
            base: self,
            options: ExecuteOptions::default(),
        }
    }
}

impl<T> TransferInterface for T where T: AbstractNameService + AccountIdentification + Execution {}

/**
    API for transferring funds to and from the account.

    # Example
    ```
    use abstract_sdk::prelude::*;
    # use cosmwasm_std::testing::mock_dependencies;
    # use abstract_sdk::mock_module::MockModule;
    # let module = MockModule::new();
    # let deps = mock_dependencies();

    let bank: Bank<MockModule>  = module.bank(deps.as_ref());
    ```
*/
pub struct Bank<'a, T: TransferInterface> {
    base: &'a mut T,
    options: ExecuteOptions,
}

impl<'a, T: TransferInterface> Bank<'a, T> {
    /// Registers the reply that will be used when executing the message
    pub fn with_reply(mut self, reply_on: ReplyOn, id: u64) -> Self {
        self.options.reply = Some(ReplyOptions {
            reply_on,
            id,
            with_data: false,
        });
        self
    }
    /// Allows for reflecting the data on the reply handler
    pub fn with_reply_and_data(mut self, reply_on: ReplyOn, id: u64) -> Self {
        self.options.reply = Some(ReplyOptions {
            reply_on,
            id,
            with_data: true,
        });
        self
    }

    /// Get the balances of the provided assets.
    pub fn balances(&self, assets: &[AssetEntry]) -> AbstractSdkResult<Vec<Asset>> {
        assets
            .iter()
            .map(|asset| self.balance(asset))
            .collect::<AbstractSdkResult<Vec<Asset>>>()
    }
    /// Get the balance of the provided asset.
    pub fn balance(&self, asset: &AssetEntry) -> AbstractSdkResult<Asset> {
        let resolved_info = asset.resolve(&self.base.deps().querier, &self.base.ans_host()?)?;
        let balance =
            resolved_info.query_balance(&self.base.deps().querier, self.base.proxy_address()?)?;
        Ok(Asset::new(resolved_info, balance))
    }

    /// Transfer the provided funds from the Account to the recipient.
    /// ```
    /// # use cosmwasm_std::{Addr, Response, Deps, DepsMut, MessageInfo};
    /// # use abstract_core::objects::AnsAsset;
    /// # use abstract_core::objects::ans_host::AnsHost;
    /// # use abstract_sdk::{
    /// #    features::{AccountIdentification, AbstractNameService, ModuleIdentification},
    /// #    TransferInterface, AbstractSdkResult, Execution,
    /// # };
    /// # struct MockModule;
    /// # impl AccountIdentification for MockModule {
    /// #    fn proxy_address(&self, _deps: Deps) -> AbstractSdkResult<Addr> {
    /// #       unimplemented!("Not needed for this example")
    /// #   }
    /// # }
    /// #
    /// # impl ModuleIdentification for MockModule {
    /// #   fn module_id(&self) -> &'static str {
    /// #      "mock_module"
    /// #  }
    /// # }
    /// #
    /// # impl AbstractNameService for MockModule {
    /// #   fn ans_host(&self, _deps: Deps) -> AbstractSdkResult<AnsHost> {
    /// #     unimplemented!("Not needed for this example")
    /// #  }
    /// # }
    /// fn transfer_asset_to_sender(app: MockModule, deps: DepsMut, info: MessageInfo, requested_asset: AnsAsset) -> AbstractSdkResult<Response> {
    ///     let bank = app.bank(deps.as_ref());
    ///     let executor = app.executor(deps.as_ref());    
    ///     let transfer_action = bank.transfer(vec![requested_asset.clone()], &info.sender)?;
    ///
    ///     let transfer_msg = executor.execute(vec![transfer_action])?;
    ///
    ///     Ok(Response::new()
    ///         .add_message(transfer_msg)
    ///         .add_attribute("recipient", info.sender)
    ///         .add_attribute("asset_sent", requested_asset.to_string()))
    /// }
    /// ```

    pub fn transfer<R: Transferable>(
        &mut self,
        funds: Vec<R>,
        recipient: &Addr,
    ) -> AbstractSdkResult<()> {
        let transferable_funds = funds
            .into_iter()
            .map(|asset| asset.transferable_asset(self.base, self.base.deps()))
            .collect::<AbstractSdkResult<Vec<Asset>>>()?;
        let msgs = transferable_funds
            .iter()
            .map(|asset| asset.transfer_msg(recipient.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        self.base
            .executor()
            .execute_with_options(msgs, self.options.clone())
    }

    /// Move funds from the contract into the Account.
    pub fn deposit<R: Transferable>(&mut self, funds: Vec<R>) -> AbstractSdkResult<()> {
        let recipient = self.base.proxy_address()?;
        let transferable_funds = funds
            .into_iter()
            .map(|asset| asset.transferable_asset(self.base, self.base.deps()))
            .collect::<AbstractSdkResult<Vec<Asset>>>()?;
        let msgs = transferable_funds
            .iter()
            .map(|asset| asset.transfer_msg(recipient.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        self.base.push_app_messages(msgs);
        Ok(())
    }

    /// Withdraw funds from the Account to this contract.
    pub fn withdraw<R: Transferable>(&mut self, env: &Env, funds: Vec<R>) -> AbstractSdkResult<()> {
        let recipient = &env.contract.address;
        self.transfer(funds, recipient)
    }

    /// Move cw20 assets from the Account to a recipient with the possibility using the cw20 send/receive hook
    ///
    /// Note:  **Native coins are NOT and will NEVER be supported by this method**.
    ///
    /// In order to send funds with your message, you need to construct the message yourself
    pub fn send<R: Transferable, M: Serialize>(
        &mut self,
        funds: R,
        recipient: &Addr,
        message: &M,
    ) -> AbstractSdkResult<()> {
        let transferable_funds = funds.transferable_asset(self.base, self.base.deps())?;

        let msg = transferable_funds.send_msg(recipient, to_json_binary(message)?)?;

        self.base.executor().execute(vec![msg])
    }
}

/// Turn an object that represents an asset into the blockchain representation of an asset, i.e. [`Asset`].
pub trait Transferable {
    /// Turn an object that represents an asset into the blockchain representation of an asset, i.e. [`Asset`].
    fn transferable_asset<T: AbstractNameService>(
        self,
        base: &T,
        deps: Deps,
    ) -> AbstractSdkResult<Asset>;
}

impl Transferable for &AnsAsset {
    fn transferable_asset<T: AbstractNameService>(
        self,
        base: &T,
        deps: Deps,
    ) -> AbstractSdkResult<Asset> {
        self.resolve(&deps.querier, &base.ans_host()?)
    }
}

impl Transferable for AnsAsset {
    fn transferable_asset<T: AbstractNameService>(
        self,
        base: &T,
        deps: Deps,
    ) -> AbstractSdkResult<Asset> {
        self.resolve(&deps.querier, &base.ans_host()?)
    }
}

impl Transferable for Asset {
    fn transferable_asset<T: AbstractNameService>(
        self,
        _base: &T,
        _deps: Deps,
    ) -> AbstractSdkResult<Asset> {
        Ok(self)
    }
}

impl Transferable for Coin {
    fn transferable_asset<T: AbstractNameService>(
        self,
        _base: &T,
        _deps: Deps,
    ) -> AbstractSdkResult<Asset> {
        Ok(Asset::from(self))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::mock_module::*;
    use abstract_testing::prelude::*;
    use cosmwasm_std::{testing::*, *};
    use speculoos::prelude::*;
    pub type ExecutionEnv<'a> = (DepsMut<'a>, Env, MessageInfo);
    use crate::base::features::ResponseGenerator;

    mod transfer_coins {
        use abstract_core::proxy::ExecuteMsg;

        use super::*;

        #[test]
        fn transfer_asset_to_sender() {
            let mut deps = mock_dependencies();
            let mut app = MockModule::new((deps.as_mut(), mock_env(), mock_info("sender", &[])));

            // ANCHOR: transfer
            let recipient: Addr = Addr::unchecked("recipient");
            let mut bank: Bank<'_, MockModule<ExecutionEnv>> = app.bank();
            let coins: Vec<Coin> = coins(100u128, "asset");
            /*let bank_transfer = */
            bank.transfer(coins.clone(), &recipient).unwrap();
            // ANCHOR_END: transfer

            let response: Response = app._generate_response().unwrap();

            let expected_msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: recipient.to_string(),
                amount: coins,
            });

            assert_that!(response.messages[0].msg).is_equal_to(
                &wasm_execute(
                    TEST_PROXY,
                    &ExecuteMsg::ModuleAction {
                        msgs: vec![expected_msg],
                    },
                    vec![],
                )
                .unwrap()
                .into(),
            );
        }
    }

    // transfer must be tested via integration test

    mod deposit {
        use super::*;

        #[test]
        fn deposit() {
            let mut deps = mock_dependencies();
            let mut app = MockModule::new((deps.as_mut(), mock_env(), mock_info("sender", &[])));

            // ANCHOR: deposit
            // Get bank API struct from the app
            let mut bank: Bank<'_, MockModule<ExecutionEnv>> = app.bank();
            // Create coins to deposit
            let coins: Vec<Coin> = coins(100u128, "asset");
            // Construct messages for deposit (transfer from this contract to the account)
            bank.deposit(coins.clone()).unwrap();
            // ANCHOR_END: deposit
            // Add to response
            let response: Response = app._generate_response().unwrap();

            let expected_msg: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
                to_address: TEST_PROXY.to_string(),
                amount: coins,
            });

            assert_that!(response.messages[0].msg).is_equal_to::<CosmosMsg>(expected_msg);
        }
    }

    mod withdraw_coins {
        use crate::{features::Executable, AccountAction};

        use super::*;

        #[test]
        fn withdraw_coins() {
            let mut deps = mock_dependencies();
            let mut app = MockModule::new((deps.as_mut(), mock_env(), mock_info("sender", &[])));
            let expected_amount = 100u128;
            let env = mock_env();

            let mut bank = app.bank();
            let coins = coins(expected_amount, "asset");
            bank.withdraw(&env, coins.clone()).unwrap();

            let expected_msg: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
                to_address: env.contract.address.to_string(),
                amount: coins,
            });

            assert_that!(app.executables.0[0]).is_equal_to(Executable::AccountAction(
                AccountAction::from_vec(vec![expected_msg]),
            ));
        }
    }

    mod send_coins {
        use cw20::Cw20ExecuteMsg;
        use cw_asset::AssetError;

        use crate::{features::Executable, AbstractSdkError, AccountAction};

        use super::*;

        #[test]
        fn send_cw20() {
            let mut deps = mock_dependencies();
            let mut app = MockModule::new((deps.as_mut(), mock_env(), mock_info("sender", &[])));
            let expected_amount = 100u128;
            let expected_recipient = Addr::unchecked("recipient");

            let mut bank = app.bank();
            let hook_msg = Empty {};
            let asset = Addr::unchecked("asset");
            let coin = Asset::cw20(asset.clone(), expected_amount);
            bank.send(coin, &expected_recipient, &hook_msg).unwrap();

            let expected_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: asset.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Send {
                    contract: expected_recipient.to_string(),
                    amount: expected_amount.into(),
                    msg: to_json_binary(&hook_msg).unwrap(),
                })
                .unwrap(),
                funds: vec![],
            });

            assert_that!(app.executables.0[0]).is_equal_to(Executable::AccountAction(
                AccountAction::from_vec(vec![expected_msg]),
            ));
        }

        #[test]
        fn send_coins() {
            let mut deps = mock_dependencies();
            let mut app = MockModule::new((deps.as_mut(), mock_env(), mock_info("sender", &[])));
            let expected_amount = 100u128;
            let expected_recipient = Addr::unchecked("recipient");

            let mut bank = app.bank();
            let coin = coin(expected_amount, "asset");
            let hook_msg = Empty {};
            let actual_res = bank.send(coin, &expected_recipient, &hook_msg);

            assert_that!(actual_res.unwrap_err()).is_equal_to::<AbstractSdkError>(
                AbstractSdkError::Asset(AssetError::UnavailableMethodForNative {
                    method: "send".into(),
                }),
            );
        }
    }
}
