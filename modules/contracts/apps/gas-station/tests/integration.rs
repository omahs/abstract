use std::str::FromStr;

use cosmwasm_std::coins;
use cw_asset::AssetInfoUnchecked;
use cw_orch::anyhow;
use speculoos::prelude::*;
use cw_orch::prelude::*;

use abstract_core::{
    ans_host::ExecuteMsgFns as AnsHostExecuteMsgFns,
    app::{BaseInstantiateMsg, InstantiateMsg},
    manager::QueryMsgFns,
    objects::gov_type::GovernanceDetails,
    version_control::ExecuteMsgFns as VcExecMsgFns,
};
use abstract_gas_station_app::{contract::{GAS_STATION_APP_ID, VERSION}, GasStationApp, GasStationExecuteMsgFns, GasStationQueryMsgFns};
use abstract_gas_station_app::msg::{GasStationInstantiateMsg, GasPumpListResponse, GasStationExecuteMsg};
use abstract_interface::{Abstract, AbstractAccount, AppDeployer};
use abstract_testing::prelude::*;
use cw_orch::deploy::Deploy;
use abstract_core::app::{BaseExecuteMsg, ExecuteMsg};
use abstract_core::manager::ExecuteMsgFns;
use abstract_core::objects::AnsAsset;

struct GasStationTest<Env: CwEnv> {
    account: AbstractAccount<Env>,
    abstr: Abstract<Env>,
    gas_station: GasStationApp<Env>,
    tube: Env,
}

impl GasStationTest<OsmosisTestTube> {
    fn setup(initial_balance: Option<Vec<Coin>>) -> anyhow::Result<Self> {
        // Download the adapter wasm
        // Create the OsmosisTestTube
        let tube = OsmosisTestTube::new(initial_balance.unwrap_or(coins(1_000_000_000_000, GAS_DENOM)));

        let abstr = Abstract::deploy_on(tube.clone(), tube.sender().to_string()).unwrap();

        let gas_station = deploy_gas_station(&tube);

        let account = setup_new_account(&abstr, TEST_NAMESPACE)?;
        setup_default_assets(&abstr);
        account.install_module(
            GAS_STATION_APP_ID,
            &InstantiateMsg {
                base: BaseInstantiateMsg {
                    ans_host_address: abstr.ans_host.addr_str()?,
                },
                module: GasStationInstantiateMsg {

                }
            },
            None,
        )?;

        let modules = account.manager.module_infos(None, None)?;
        gas_station.set_address(&modules.module_infos[0].address);

        Ok(Self {
            tube,
            account,
            abstr,
            gas_station,
        })
    }
}
// impl GasStationTest<Mock> {
//     fn setup(initial_balance: Option<Vec<Coin>>) -> anyhow::Result<Self> {
//         // Download the adapter wasm
//         // Create the OsmosisTestTube
//         let tube = Mock::new(&Addr::unchecked(TEST_OWNER));
//
//         let abstr = Abstract::deploy_on(tube.clone(), tube.sender().to_string()).unwrap();
//
//         let gas_station = deploy_gas_station(&tube);
//
//         let account = setup_new_account(&abstr, TEST_NAMESPACE)?;
//         setup_default_assets(&abstr);
//         account.install_module(
//             GAS_STATION_APP_ID,
//             &InstantiateMsg {
//                 base: BaseInstantiateMsg {
//                     ans_host_address: abstr.ans_host.addr_str()?,
//                 },
//                 module: GasStationInstantiateMsg {
//
//                 }
//             },
//             None,
//         )?;
//
//         let modules = account.manager.module_infos(None, None)?;
//         gas_station.set_address(&modules.module_infos[0].address);
//
//         Ok(Self {
//             tube,
//             account,
//             abstr,
//             gas_station,
//         })
//     }
// }

fn setup_default_assets<Env: CwEnv>(abstr: &Abstract<Env>) {
    // register juno as an asset
    abstr
        .ans_host
        .update_asset_addresses(
            vec![(
                GAS_ANS_ID.to_string(),
                AssetInfoUnchecked::from_str(&format!("native:{}", GAS_DENOM)).unwrap(),
            )],
            vec![],
        )
        .unwrap();
}


// Uploads and returns the giftcard issuer
fn deploy_gas_station<Env: CwEnv>(tube: &Env) -> GasStationApp<Env> {
    let station = GasStationApp::new(GAS_STATION_APP_ID, tube.clone());

    // deploy the abstract gas station
    station
        .deploy(VERSION.parse().unwrap())
        .unwrap();

    station
}


const GAS_DENOM: &'static str = "uosmo";
const GAS_ANS_ID: &'static str = "osmo>osmo";

// Returns an account with the necessary setup
fn setup_new_account<Env: CwEnv>(
    abstr_deployment: &Abstract<Env>,
    namespace: impl ToString,
) -> anyhow::Result<AbstractAccount<Env>> {
    // TODO: might need to move this
    let signing_account = abstr_deployment.account_factory.get_chain().sender();

    // Create a new account to install the app onto
    let account = abstr_deployment
        .account_factory
        .create_default_account(GovernanceDetails::Monarchy {
            monarch: signing_account.into_string(),
        })
        .unwrap();

    // claim the namespace so app can be deployed
    abstr_deployment
        .version_control
        .claim_namespace(account.id().unwrap(), namespace.to_string())
        .unwrap();

    // register base asset!
    // account.proxy.call_as(&abstr_deployment.account_factory.get_chain().sender).update_assets(vec![(AssetEntry::from(ISSUE_ASSET), UncheckedPriceSource::None)], vec![]).unwrap();

    Ok(account)
}

#[test]
fn successful_install_with_no_pumps() -> anyhow::Result<()> {
    // Set up the environment and contract
    let test_env = GasStationTest::setup(None)?;

    let pump_list: GasPumpListResponse = test_env.gas_station.gas_pump_list()?;
    assert_that!(
        pump_list.pumps).is_empty();
    Ok(())
}

#[test]
fn create_pump() -> anyhow::Result<()> {
    // Set up the environment and contract
    let test_env = GasStationTest::setup(None)?;

    let fuel_mix: Vec<AnsAsset> = vec![AnsAsset::new(GAS_ANS_ID.to_string(), 1_000_000_000_000u128)];

    // emulate exec_on_module
    let create_pump_res = test_env.account.manager.execute_on_module(GAS_STATION_APP_ID, ExecuteMsg::<GasStationExecuteMsg, Empty>::from(GasStationExecuteMsg::CreateGasPump {
        fuel_mix,
        grade: "osmo".to_string(),
    }))?;
    // let create_pump_res = test_env.gas_station.create_gas_pump(fuel_mix, "osmo".to_string())?;


    Ok(())
}

