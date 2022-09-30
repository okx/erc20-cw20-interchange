# ERC20资产跨虚拟机转移成CW资产
## 1、部署合约
跨虚拟机转移资产需在两个虚拟机上分别部署合约，若在EVM上已存在代币，需部署`Bridge.sol`合约，若无代币，可直接部署`BridgeERC20.sol`。以下以`BridgeERC20.sol`为例,网络为本地。
### 部署EVM合约
使用hardhat部署：

hardhat.config.js
```javascript
/** @type import('hardhat/config').HardhatUserConfig */
require("@nomiclabs/hardhat-waffle");

module.exports = {
  solidity: "0.8.17",
  networks: {
    hardhat: {
    },
    localOKC: {
      url: "http://localhost:8545",
      accounts: ["私钥"]
    }
  }
};
```
deployEVMCotract.js
```javascript
const hre = require("hardhat");

async function main() {

  const BridgeERC20 = await hre.ethers.getContractFactory("BridgeERC20");
  const bridgeERC20 = await BridgeERC20.deploy("testERC20","T20", 10000000000);

  console.log("合约地址：" + bridgeERC20.address)
  console.log("账户余额：" + await bridgeERC20.balanceOf("0x83D83497431C2D3FEab296a9fba4e5FaDD2f7eD0"))
}

// We recommend this pattern to be able to use async/await everywhere
// and properly handle errors.
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
```
#### 脚本部署
    npx hardhat run --network localOKC scripts/deployEVMCotract.js
    //return
    合约地址：0xB1cC8e7F89CF7DD0A8F1B3fB8a0295e3E72Ad2fD
    账户余额：10000000000
得到合约ex地址：

    oker@192 ~ % exchaincli addr convert 0xB1cC8e7F89CF7DD0A8F1B3fB8a0295e3E72Ad2fD
    Bech32 format with prefix <okexchain>: okexchain1k8xgulufea7ap283k0ac5q54u0nj45havzlxep
    Bech32 format with prefix <ex>: ex1k8xgulufea7ap283k0ac5q54u0nj45hatf5x6d
    Hex format with prefix <0x>: 0xB1cC8e7F89CF7DD0A8F1B3fB8a0295e3E72Ad2fD
#### 初始化合约
需要部署CM合约获得合约地址后操作
调用initialize方法传入CM合约地址

    function initialize(string calldata _wasmContractAddress)

### 部署CM合约
添加EVM地址到CM合约中

    pub const PREFIX_CONFIG: &[u8] = b"config";
    pub const PREFIX_BALANCES: &[u8] = b"balances";
    pub const PREFIX_ALLOWANCES: &[u8] = b"allowances";

    pub const KEY_CONSTANTS: &[u8] = b"constants";
    pub const KEY_TOTAL_SUPPLY: &[u8] = b"total_supply";
    pub const EVM_CONTRACT_ADDR: &str = "ex1k8xgulufea7ap283k0ac5q54u0nj45hatf5x6d";

#### 编译合约
    oker@192 bridgeERC20 % RUSTFLAGS='-C link-arg=-s' cargo wasm
    Compiling cw-erc20 v0.10.0 (/Users/oker/Desktop/job/VMTokenBridge/cosmos/bridgeERC20)
        Finished release [optimized] target(s) in 5.49s

#### 上传合约
    oker@192 bridgeERC20 % exchaincli tx wasm upload ./target/wasm32-unknown-unknown/release/cw_erc20.wasm --fees 0.01okt --from ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9 --gas=20000000 -b block -y
    {
    "height": "16793",
    "txhash": "C0A4B93E55E4761B62859A342CC24C50F27E2F560DC57440856EC88ADEAA63F0",
    "data": "0806",
    "raw_log": "[{\"msg_index\":0,\"log\":\"\",\"events\":[{\"type\":\"message\",\"attributes\":[{\"key\":\"action\",\"value\":\"store-code\"},{\"key\":\"module\",\"value\":\"wasm\"},{\"key\":\"sender\",\"value\":\"ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9\"}]},{\"type\":\"store_code\",\"attributes\":[{\"key\":\"code_id\",\"value\":\"6\"}]}]}]",
    "logs": [
        {
        "msg_index": 0,
        "log": "",
        "events": [
            {
            "type": "message",
            "attributes": [
                {
                "key": "action",
                "value": "store-code"
                },
                {
                "key": "module",
                "value": "wasm"
                },
                {
                "key": "sender",
                "value": "ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9"
                }
            ]
            },
            {
            "type": "store_code",
            "attributes": [
                {
                "key": "code_id",
                "value": "6"
                }
            ]
            }
        ]
        }
    ],
    "gas_wanted": "20000000",
    "gas_used": "1298912"
    }
#### 初始化合约
    oker@192 bridgeERC20 % exchaincli tx wasm instantiate "6" '{"decimals":10,"name":"test ERC20", "symbol":"TERC"}' --label " "  --admin ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9 --fees 0.001okt --from ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9 -b block -y
    {
    "height": "16849",
    "txhash": "CF34B1D8236F284A07BB52A6469A2581C4DDDB9B469B51C83C27B1371DE76EB3",
    "data": "0A3D6578316D663670746B73736464666D787668647830656368306B30336B7470366B6639796B353972656E61753267766874336E71326771726538656371",
    "raw_log": "[{\"msg_index\":0,\"log\":\"\",\"events\":[{\"type\":\"instantiate\",\"attributes\":[{\"key\":\"_contract_address\",\"value\":\"ex1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqre8ecq\"},{\"key\":\"code_id\",\"value\":\"6\"}]},{\"type\":\"message\",\"attributes\":[{\"key\":\"action\",\"value\":\"instantiate\"},{\"key\":\"module\",\"value\":\"wasm\"},{\"key\":\"sender\",\"value\":\"ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9\"}]}]}]",
    "logs": [
        {
        "msg_index": 0,
        "log": "",
        "events": [
            {
            "type": "instantiate",
            "attributes": [
                {
                "key": "_contract_address",
                "value": "ex1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqre8ecq"
                },
                {
                "key": "code_id",
                "value": "6"
                }
            ]
            },
            {
            "type": "message",
            "attributes": [
                {
                "key": "action",
                "value": "instantiate"
                },
                {
                "key": "module",
                "value": "wasm"
                },
                {
                "key": "sender",
                "value": "ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9"
                }
            ]
            }
        ]
        }
    ],
    "gas_wanted": "200000",
    "gas_used": "165059"
    }

获得合约地址：`ex1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqre8ecq`

## 2、资产跨链
`BridgeERC20.sol`合约实现了`send_to_wasm`方法和`mintERC20`方法用于发送和接收CM的资产转移消息。CM合约同样实现了`send_to_evm`和`MintCW20`方法用于发送和接收EVM的资产。
### ERC20 => CW20

`function send_to_wasm

(ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9,ex1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqre8ecq,100)`

    oker@192 bridgeERC20 % exchaincli query wasm contract-state smart "ex1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqre8ecq" '{"balance":{"address":"ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9"}}'
    {"data":{"balance":"100"}}

`function balanceOf(0x83D83497431C2D3FEab296a9fba4e5FaDD2f7eD0)

//return

9999999900`

### CW20 => ERC20

    oker@192 bridgeERC20 % exchaincli tx wasm execute "ex1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqre8ecq" '{"send_to_evm":{"contract":"ex1k8xgulufea7ap283k0ac5q54u0nj45hatf5x6d","recipient":"ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9","amount":"99"}}' --fees 0.001okt --from ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9 -b block -y
    {
    "height": "17556",
    "txhash": "6EF59F09AACDB9635EFEA493A1C0CDB6AEC3B95F1A3F922DDCE0843F32731E4E",
    "data": "0A0F74686520726573756C742064617461",
    "raw_log": "[{\"msg_index\":0,\"log\":\"\",\"events\":[{\"type\":\"execute\",\"attributes\":[{\"key\":\"_contract_address\",\"value\":\"ex1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqre8ecq\"}]},{\"type\":\"message\",\"attributes\":[{\"key\":\"action\",\"value\":\"execute\"},{\"key\":\"module\",\"value\":\"wasm\"},{\"key\":\"sender\",\"value\":\"ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9\"}]},{\"type\":\"wasm\",\"attributes\":[{\"key\":\"_contract_address\",\"value\":\"ex1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqre8ecq\"},{\"key\":\"action\",\"value\":\"call evm\"},{\"key\":\"amount\",\"value\":\"99\"}]}]}]",
    "logs": [
        {
        "msg_index": 0,
        "log": "",
        "events": [
            {
            "type": "execute",
            "attributes": [
                {
                "key": "_contract_address",
                "value": "ex1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqre8ecq"
                }
            ]
            },
            {
            "type": "message",
            "attributes": [
                {
                "key": "action",
                "value": "execute"
                },
                {
                "key": "module",
                "value": "wasm"
                },
                {
                "key": "sender",
                "value": "ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9"
                }
            ]
            },
            {
            "type": "wasm",
            "attributes": [
                {
                "key": "_contract_address",
                "value": "ex1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqre8ecq"
                },
                {
                "key": "action",
                "value": "call evm"
                },
                {
                "key": "amount",
                "value": "99"
                }
            ]
            }
        ]
        }
    ],
    "gas_wanted": "200000",
    "gas_used": "171870"
    }

    oker@192 bridgeERC20 % exchaincli query wasm contract-state smart "ex1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqre8ecq" '{"balance":{"address":"ex1s0vrf96rrsknl64jj65lhf89ltwj7lksr7m3r9"}}'                                                                                                                                        
    {"data":{"balance":"1"}}

`function balanceOf(0x83D83497431C2D3FEab296a9fba4e5FaDD2f7eD0)

//return

9999999999`
## 3、二次开发
OKC公链在接收EVM特定事件`__OKCSendToWasm`后，会触发一笔CM交易调用指定地址`的MintCW20`方法；在接收到CM发出`CosmosMsg::Custom`消息时，会通过特定地址`0xc63cf6c8E1f3DF41085E9d8Af49584dae1432b4f`发起一笔EVM交易调用指定地址的`mintERC20`方法