//! # Represents Abstract Client
//!
//! [`AbstractClient`] allows you to do everything you might need to work with the Abstract
//! or to be more precise
//!
//! - Create or interact with Account
//! - Install or interact with a module (including apps and adapters)
//! - Publish modules
//! - Do integration tests with Abstract
//!
//! Example of publishing mock app
//!
//! ```
//! # use abstract_client::AbstractClientError;
//! use abstract_app::mock::mock_app_dependency::interface::MockAppI;
//! use cw_orch::prelude::*;
//! use abstract_client::{AbstractClient, Publisher, Namespace};
//!
//! let chain = Mock::new(&Addr::unchecked("sender"));
//! let client = AbstractClient::builder(chain).build()?;
//!
//! let namespace = Namespace::new("tester")?;
//! let publisher: Publisher<Mock> = client
//!     .publisher_builder(namespace)
//!     .build()?;
//!
//! publisher.publish_app::<MockAppI<Mock>>()?;
//! # Ok::<(), AbstractClientError>(())
//! ```

use abstract_core::objects::{namespace::Namespace, AccountId};
use abstract_interface::{Abstract, AbstractAccount, AnsHost, ManagerQueryFns, VersionControl};
use cosmwasm_std::{Addr, BlockInfo, Coin, Empty, Uint128};
use cw_orch::prelude::*;

use crate::{
    account::{Account, AccountBuilder},
    source::AccountSource,
    AbstractClientError, Environment, PublisherBuilder,
};

/// Client to interact with Abstract accounts and modules
pub struct AbstractClient<Chain: CwEnv> {
    pub(crate) abstr: Abstract<Chain>,
}

/// The result type for the Abstract Client.
pub type AbstractClientResult<T> = Result<T, AbstractClientError>;

impl<Chain: CwEnv> AbstractClient<Chain> {
    /// Get [`AbstractClient`] from a chosen environment. [`Abstract`] should
    /// already be deployed to this environment.
    ///
    /// ```
    /// use abstract_client::AbstractClient;
    /// # use abstract_client::{Environment, AbstractClientError};
    /// # use cw_orch::prelude::*;
    /// # let chain = Mock::new(&Addr::unchecked("sender"));
    /// # let client = AbstractClient::builder(chain.clone()).build().unwrap(); // Deploy mock abstract
    ///
    /// let client = AbstractClient::new(chain)?;
    /// # Ok::<(), AbstractClientError>(())
    /// ```
    pub fn new(chain: Chain) -> AbstractClientResult<Self> {
        let abstr = Abstract::load_from(chain)?;
        Ok(Self { abstr })
    }

    /// Version Control contract API
    ///
    /// The Version Control contract is a database contract that stores all module-related information.
    /// ```
    /// # use abstract_client::AbstractClientError;
    /// # let chain = cw_orch::prelude::Mock::new(&Addr::unchecked("sender"));
    /// # let client = abstract_client::AbstractClient::builder(chain).build().unwrap();
    /// use abstract_core::objects::{module_reference::ModuleReference, module::ModuleInfo};
    /// // For getting version control address
    /// use cw_orch::prelude::*;
    ///
    /// let version_control = client.version_control();
    /// let vc_module = version_control.module(ModuleInfo::from_id_latest("abstract:version-control")?)?;
    /// assert_eq!(vc_module.reference, ModuleReference::Native(version_control.address()?));
    /// # Ok::<(), AbstractClientError>(())
    /// ```
    pub fn version_control(&self) -> &VersionControl<Chain> {
        &self.abstr.version_control
    }

    /// Abstract Name Service contract API
    ///
    /// The Abstract Name Service contract is a database contract that stores all asset-related information.
    /// ```
    /// # use abstract_client::AbstractClientError;
    /// use abstract_client::{AbstractClient, ClientResolve};
    /// use cw_asset::AssetInfo;
    /// use abstract_app::objects::AssetEntry;
    /// // For getting version control address
    /// use cw_orch::prelude::*;
    ///
    /// let denom = "test_denom";
    /// let entry = "denom";
    /// # let client = AbstractClient::builder(Mock::new(&Addr::unchecked("sender")))
    /// #     .asset(entry, cw_asset::AssetInfoBase::Native(denom.to_owned()))
    /// #     .build()?;
    ///
    /// let name_service = client.name_service();
    /// let asset_entry = AssetEntry::new(entry);
    /// let asset = asset_entry.resolve(name_service)?;
    /// assert_eq!(asset, AssetInfo::Native(denom.to_owned()));
    /// # Ok::<(), AbstractClientError>(())
    /// ```
    pub fn name_service(&self) -> &AnsHost<Chain> {
        &self.abstr.ans_host
    }

    /// Return current block info see [`BlockInfo`].
    pub fn block_info(&self) -> AbstractClientResult<BlockInfo> {
        self.environment()
            .block_info()
            .map_err(|e| AbstractClientError::CwOrch(e.into()))
    }

    /// Publisher builder for creating new [`Publisher`](crate::Publisher) Abstract Account
    /// To publish any modules your account requires to have claimed a namespace.
    pub fn publisher_builder(&self, namespace: Namespace) -> PublisherBuilder<Chain> {
        PublisherBuilder::new(AccountBuilder::new(&self.abstr), namespace)
    }

    /// Builder for creating a new Abstract [`Account`].
    pub fn account_builder(&self) -> AccountBuilder<Chain> {
        AccountBuilder::new(&self.abstr)
    }

    /// Address of the sender
    pub fn sender(&self) -> Addr {
        self.environment().sender()
    }

    /// Fetch an [`Account`] from a given source.
    ///
    /// This method is used to retrieve an account from a given source. It will **not** create a new account if the source is invalid.
    ///
    /// Sources that can be used are:
    /// - [`Namespace`]: Will retrieve the account from the namespace if it is already claimed.
    /// - [`AccountId`]: Will retrieve the account from the account id.
    /// - App [`Addr`]: Will retrieve the account from an app that is installed on it.
    pub fn account_from<T: Into<AccountSource>>(
        &self,
        source: T,
    ) -> AbstractClientResult<Account<Chain>> {
        let source = source.into();
        let chain = self.abstr.version_control.get_chain();

        match source {
            AccountSource::Namespace(namespace) => {
                // if namespace, check if we need to claim or not.
                // Check if namespace already claimed
                let account_from_namespace_result: Option<Account<Chain>> =
                    Account::maybe_from_namespace(&self.abstr, namespace.clone(), true)?;

                // Only return if the account can be retrieved without errors.
                if let Some(account_from_namespace) = account_from_namespace_result {
                    Ok(account_from_namespace)
                } else {
                    Err(AbstractClientError::NamespaceNotClaimed {
                        namespace: namespace.to_string(),
                    })
                }
            }
            AccountSource::AccountId(account_id) => {
                let abstract_account: AbstractAccount<Chain> =
                    AbstractAccount::new(&self.abstr, account_id.clone());
                Ok(Account::new(abstract_account, true))
            }
            AccountSource::App(app) => {
                // Query app for manager address and get AccountId from it.
                let app_config: abstract_core::app::AppConfigResponse = chain
                    .query(
                        &abstract_core::app::QueryMsg::<Empty>::Base(
                            abstract_core::app::BaseQueryMsg::BaseConfig {},
                        ),
                        &app,
                    )
                    .map_err(Into::into)?;

                let manager_config: abstract_core::manager::ConfigResponse = chain
                    .query(
                        &abstract_core::manager::QueryMsg::Config {},
                        &app_config.manager_address,
                    )
                    .map_err(Into::into)?;
                // This function verifies the account-id is valid and returns an error if not.
                let abstract_account: AbstractAccount<Chain> =
                    AbstractAccount::new(&self.abstr, manager_config.account_id);
                Ok(Account::new(abstract_account, true))
            }
        }
    }

    /// Retrieve denom balance for provided address
    pub fn query_balance(
        &self,
        address: &Addr,
        denom: impl Into<String>,
    ) -> AbstractClientResult<Uint128> {
        let coins = self
            .environment()
            .bank_querier()
            .balance(address, Some(denom.into()))
            .map_err(Into::into)?;
        // There will always be a single element in this case.
        Ok(coins[0].amount)
    }

    /// Retrieve balances of all denoms for provided address
    pub fn query_balances(&self, address: &Addr) -> AbstractClientResult<Vec<Coin>> {
        self.environment()
            .bank_querier()
            .balance(address, None)
            .map_err(Into::into)
            .map_err(Into::into)
    }

    /// Waits for a specified number of blocks.
    pub fn wait_blocks(&self, amount: u64) -> AbstractClientResult<()> {
        self.environment()
            .wait_blocks(amount)
            .map_err(Into::into)
            .map_err(Into::into)
    }

    /// Waits for a specified number of blocks.
    pub fn wait_seconds(&self, secs: u64) -> AbstractClientResult<()> {
        self.environment()
            .wait_seconds(secs)
            .map_err(Into::into)
            .map_err(Into::into)
    }

    /// Waits for next block.
    pub fn next_block(&self) -> AbstractClientResult<()> {
        self.environment()
            .next_block()
            .map_err(Into::into)
            .map_err(Into::into)
    }

    // Retrieve the last account created by the client.
    /// Returns `None` if no account has been created yet.
    /// **Note**: This only returns accounts that were created with the Client. Any accounts created through the web-app will not be returned.
    pub fn get_last_account(&self) -> AbstractClientResult<Option<Account<Chain>>> {
        let addresses = self.environment().state().get_all_addresses()?;
        // Now search for all the keys that start with "abstract:manager-x" and return the one which has the highest x.
        let mut last_account: Option<(u32, Account<Chain>)> = None;
        for id in addresses.keys() {
            let Some(account_id) = is_local_manager(id.as_str())? else {
                continue;
            };

            // only take accounts that the current sender owns
            let account = AbstractAccount::new(&self.abstr, account_id.clone());
            if account.manager.ownership()?.owner != Some(self.environment().sender().to_string()) {
                continue;
            }

            if let Some((last_account_id, _)) = last_account {
                if account_id.seq() > last_account_id {
                    last_account = Some((account_id.seq(), Account::new(account, true)));
                }
            } else {
                last_account = Some((account_id.seq(), Account::new(account, true)));
            }
        }
        Ok(last_account.map(|(_, account)| account))
    }
}

pub(crate) fn is_local_manager(id: &str) -> AbstractClientResult<Option<AccountId>> {
    if !id.starts_with(abstract_core::MANAGER) {
        return Ok(None);
    }

    let (_, account_id_str) = id.split_once('-').unwrap();
    let account_id = AccountId::try_from(account_id_str)?;

    // Only take local accounts into account.
    if account_id.is_remote() {
        return Ok(None);
    }

    Ok(Some(account_id))
}

#[cfg(test)]
mod tests {
    use cw_orch::mock::Mock;

    use super::*;

    #[test]
    fn local_account() {
        let result = is_local_manager("abstract:manager-local-9");
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn remote_account() {
        let result = is_local_manager("abstract:manager-eth>btc-9");
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn not_manager() {
        let result = is_local_manager("abstract:proxy-local-9");
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn last_owned_abstract_account() {
        let sender = Addr::unchecked("sender");
        let chain = Mock::new(&sender);
        Abstract::deploy_on(chain.clone(), sender.to_string()).unwrap();

        let client = AbstractClient::new(chain).unwrap();
        let _acc = client.account_builder().build().unwrap();
        let acc_2 = client.account_builder().build().unwrap();

        let other_owner = Addr::unchecked("other_owner");
        // create account with sender as sender but other owner
        client
            .account_builder()
            .ownership(
                abstract_core::objects::gov_type::GovernanceDetails::Monarchy {
                    monarch: other_owner.to_string(),
                },
            )
            .build()
            .unwrap();

        let last_account = client.get_last_account().unwrap().unwrap();

        assert_eq!(acc_2.id().unwrap(), last_account.id().unwrap());
    }
}
