/**
* This file was automatically generated by @abstract-money/ts-codegen@0.28.3.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @abstract-money/ts-codegen generate command to regenerate this file.
*/

import { Coin } from "@cosmjs/amino";
import { MsgExecuteContractEncodeObject } from "cosmwasm";
import { MsgExecuteContract } from "cosmjs-types/cosmwasm/wasm/v1/tx";
import { toUtf8 } from "@cosmjs/encoding";
import { AppExecuteMsg, AppExecuteMsgFactory } from "@abstract-money/abstract.js";
import { InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, ConfigResponse } from "./Template.types";
export interface TemplateMessage {
  contractAddress: string;
  sender: string;
  updateConfig: (_funds?: Coin[]) => MsgExecuteContractEncodeObject;
}
export class TemplateMessageComposer implements TemplateMessage {
  sender: string;
  contractAddress: string;

  constructor(sender: string, contractAddress: string) {
    this.sender = sender;
    this.contractAddress = contractAddress;
    this.updateConfig = this.updateConfig.bind(this);
  }

  updateConfig = (_funds?: Coin[]): MsgExecuteContractEncodeObject => {
    const msg = {
      update_config: {}
    };
    const moduleMsg: AppExecuteMsg<ExecuteMsg> = AppExecuteMsgFactory.executeApp(msg);
    return {
      typeUrl: "/cosmwasm.wasm.v1.MsgExecuteContract",
      value: MsgExecuteContract.fromPartial({
        sender: this.sender,
        contract: this.contractAddress,
        msg: toUtf8(JSON.stringify(moduleMsg)),
        funds: _funds
      })
    };
  };
}