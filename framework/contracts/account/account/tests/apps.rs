use abstract_app::{gen_app_mock, mock, mock::MockError};
use abstract_integration_tests::{create_default_account, AResult};
use abstract_interface::*;
use abstract_manager::error::ManagerError;
use abstract_std::{
    manager::ModuleInstallConfig,
    objects::{
        gov_type::TopLevelOwnerResponse,
        module::{ModuleInfo, ModuleStatus, ModuleVersion},
        AccountId,
    },
    version_control::ModuleFilter,
    ACCOUNT,
};
use abstract_testing::prelude::*;
use cosmwasm_std::{coin, CosmosMsg};
use cw_controllers::{AdminError, AdminResponse};
use cw_orch::prelude::*;
use speculoos::prelude::*;

const APP_ID: &str = "tester:app";
const APP_VERSION: &str = "1.0.0";
gen_app_mock!(MockApp, APP_ID, APP_VERSION, &[]);

#[test]
fn execute_on_proxy_through_manager() -> AResult {
    let chain = MockBech32::new("mock");
    let sender = chain.sender_addr();
    let deployment = Abstract::deploy_on(chain.clone(), sender.to_string())?;
    let account = create_default_account(&deployment.account_factory)?;

    // mint coins to proxy address
    chain.set_balance(&account.proxy.address()?, vec![Coin::new(100_000, TTOKEN)])?;
    // mint other coins to owner
    chain.set_balance(&sender, vec![Coin::new(100, "other_coin")])?;

    // burn coins from proxy
    let proxy_balance = chain
        .app
        .borrow()
        .wrap()
        .query_all_balances(account.proxy.address()?)?;
    assert_that!(proxy_balance).is_equal_to(vec![Coin::new(100_000, TTOKEN)]);

    let burn_amount: Vec<Coin> = vec![Coin::new(10_000, TTOKEN)];
    let forwarded_coin: Coin = coin(100, "other_coin");

    account.account.exec_on_module(
        cosmwasm_std::to_json_binary(&abstract_std::proxy::ExecuteMsg::ModuleAction {
            msgs: vec![CosmosMsg::Bank(cosmwasm_std::BankMsg::Burn {
                amount: burn_amount,
            })],
        })?,
        ACCOUNT.to_string(),
        &[forwarded_coin.clone()],
    )?;

    let proxy_balance = chain
        .app
        .borrow()
        .wrap()
        .query_all_balances(account.proxy.address()?)?;
    assert_that!(proxy_balance)
        .is_equal_to(vec![forwarded_coin, Coin::new(100_000 - 10_000, TTOKEN)]);

    take_storage_snapshot!(chain, "execute_on_proxy_through_manager");

    Ok(())
}

#[test]
fn account_install_app() -> AResult {
    let chain = MockBech32::new("mock");
    let sender = chain.sender_addr();
    Abstract::deploy_on(chain.clone(), sender.to_string())?;
    abstract_integration_tests::manager::account_install_app(chain.clone())?;
    take_storage_snapshot!(chain, "account_install_app");
    Ok(())
}

#[test]
fn account_app_ownership() -> AResult {
    let chain = MockBech32::new("mock");
    let sender = chain.sender_addr();
    let deployment = Abstract::deploy_on(chain.clone(), sender.to_string())?;
    let account = create_default_account(&deployment.account_factory)?;

    deployment
        .version_control
        .claim_namespace(TEST_ACCOUNT_ID, "tester".to_owned())?;

    let app = MockApp::new_test(chain.clone());
    app.deploy(APP_VERSION.parse().unwrap(), DeployStrategy::Try)?;
    account.install_app(&app, &MockInitMsg {}, None)?;

    let admin_res: AdminResponse =
        app.query(&mock::QueryMsg::Base(app::BaseQueryMsg::BaseAdmin {}))?;
    assert_eq!(admin_res.admin.unwrap(), account.account.address()?);

    // Can call either by account owner or manager
    app.call_as(&sender).execute(
        &mock::ExecuteMsg::Module(MockExecMsg::DoSomethingAdmin {}),
        None,
    )?;
    app.call_as(&account.account.address()?).execute(
        &mock::ExecuteMsg::Module(MockExecMsg::DoSomethingAdmin {}),
        None,
    )?;

    // Not admin or manager
    let err: MockError = app
        .call_as(&Addr::unchecked("who"))
        .execute(
            &mock::ExecuteMsg::Module(MockExecMsg::DoSomethingAdmin {}),
            None,
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, MockError::Admin(AdminError::NotAdmin {}));
    Ok(())
}

#[test]
fn subaccount_app_ownership() -> AResult {
    let chain = MockBech32::new("mock");
    let sender = chain.sender_addr();
    let deployment = Abstract::deploy_on(chain.clone(), sender.to_string())?;
    let account = create_default_account(&deployment.account_factory)?;

    deployment
        .version_control
        .claim_namespace(TEST_ACCOUNT_ID, "tester".to_owned())?;

    let app = MockApp::new_test(chain.clone());
    app.deploy(APP_VERSION.parse().unwrap(), DeployStrategy::Try)?;

    account.account.create_sub_account(
        vec![ModuleInstallConfig::new(
            ModuleInfo::from_id_latest(APP_ID).unwrap(),
            Some(to_json_binary(&MockInitMsg {}).unwrap()),
        )],
        "My subaccount".to_string(),
        None,
        None,
        None,
        None,
        &[],
    )?;

    let sub_account = AbstractAccount::new(&deployment, AccountId::local(2));
    let module = sub_account.account.module_info(APP_ID)?.unwrap();
    app.set_address(&module.address);

    // Check query gives us right Top Level Owner
    let top_level_owner_res: TopLevelOwnerResponse =
        app.query(&mock::QueryMsg::Base(app::BaseQueryMsg::TopLevelOwner {}))?;
    assert_eq!(top_level_owner_res.address, sender);

    let admin_res: AdminResponse =
        app.query(&mock::QueryMsg::Base(app::BaseQueryMsg::BaseAdmin {}))?;
    assert_eq!(admin_res.admin.unwrap(), sub_account.account.address()?);
    app.call_as(&sender).execute(
        &mock::ExecuteMsg::Module(MockExecMsg::DoSomethingAdmin {}),
        None,
    )?;
    Ok(())
}

#[test]
fn cant_reinstall_app_after_uninstall() -> AResult {
    let chain = MockBech32::new("mock");
    let sender = chain.sender_addr();
    let deployment = Abstract::deploy_on(chain.clone(), sender.to_string())?;
    let account = create_default_account(&deployment.account_factory)?;

    deployment
        .version_control
        .claim_namespace(TEST_ACCOUNT_ID, "tester".to_owned())?;

    let app = MockApp::new_test(chain.clone());
    app.deploy(APP_VERSION.parse().unwrap(), DeployStrategy::Try)?;
    account.install_app(&app, &MockInitMsg {}, None)?;

    // Reinstall
    account.account.uninstall_module(APP_ID.to_owned())?;
    let Err(AbstractInterfaceError::Orch(err)) = account.install_app(&app, &MockInitMsg {}, None)
    else {
        panic!("Expected error");
    };
    let manager_err: ManagerError = err.downcast().unwrap();
    assert_eq!(manager_err, ManagerError::ProhibitedReinstall {});
    Ok(())
}

#[test]
fn deploy_strategy_uploaded() -> AResult {
    let chain = MockBech32::new("mock");
    let sender = chain.sender_addr();
    let deployment = Abstract::deploy_on(chain.clone(), sender.to_string())?;
    let _account = create_default_account(&deployment.account_factory)?;

    deployment
        .version_control
        .claim_namespace(TEST_ACCOUNT_ID, "tester".to_owned())?;
    deployment
        .version_control
        .update_config(None, None, Some(false))?;

    let app = MockApp::new_test(chain.clone());
    app.upload()?;

    // Deploy try
    app.deploy(APP_VERSION.parse().unwrap(), DeployStrategy::Try)?;
    let module_list = deployment.version_control.module_list(
        Some(ModuleFilter {
            status: Some(ModuleStatus::Pending),
            ..Default::default()
        }),
        None,
        None,
    )?;
    assert!(module_list.modules[0].module.info.name == "app");

    // Clean module
    deployment.version_control.approve_or_reject_modules(
        vec![],
        vec![ModuleInfo::from_id(
            APP_ID,
            ModuleVersion::Version(APP_VERSION.to_owned()),
        )?],
    )?;

    // Deploy Error
    app.deploy(APP_VERSION.parse().unwrap(), DeployStrategy::Error)?;
    let module_list = deployment.version_control.module_list(
        Some(ModuleFilter {
            status: Some(ModuleStatus::Pending),
            ..Default::default()
        }),
        None,
        None,
    )?;
    assert!(module_list.modules[0].module.info.name == "app");

    // Clean module
    deployment.version_control.approve_or_reject_modules(
        vec![],
        vec![ModuleInfo::from_id(
            APP_ID,
            ModuleVersion::Version(APP_VERSION.to_owned()),
        )?],
    )?;

    app.deploy(APP_VERSION.parse().unwrap(), DeployStrategy::Force)?;
    let module_list = deployment.version_control.module_list(
        Some(ModuleFilter {
            status: Some(ModuleStatus::Pending),
            ..Default::default()
        }),
        None,
        None,
    )?;
    assert!(module_list.modules[0].module.info.name == "app");

    Ok(())
}

#[test]
fn deploy_strategy_deployed() -> AResult {
    let chain = MockBech32::new("mock");
    let sender = chain.sender_addr();
    let deployment = Abstract::deploy_on(chain.clone(), sender.to_string())?;
    let _account = create_default_account(&deployment.account_factory)?;

    deployment
        .version_control
        .claim_namespace(TEST_ACCOUNT_ID, "tester".to_owned())?;
    deployment
        .version_control
        .update_config(None, None, Some(false))?;

    let app = MockApp::new_test(chain.clone());

    // deploy (not approved)
    app.deploy(APP_VERSION.parse().unwrap(), DeployStrategy::Try)?;

    // Deploy try
    let try_res = app.deploy(APP_VERSION.parse().unwrap(), DeployStrategy::Try);
    assert!(try_res.is_ok());

    // Deploy Error
    let error_res = app.deploy(APP_VERSION.parse().unwrap(), DeployStrategy::Error);
    assert!(error_res.is_err());

    // Deploy Force
    let force_res = app.deploy(APP_VERSION.parse().unwrap(), DeployStrategy::Force);
    // App not updatable
    assert!(force_res.is_err());
    Ok(())
}
