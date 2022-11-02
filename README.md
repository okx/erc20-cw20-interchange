# erc20 cw20 interchange (ECI)
This project is the contract part that helps KIP20 assets on OKC be transferred to the wasm virtual machine.Users can modify the contract for deployment according to their own needs. This project only provides reference for users
## 1、before starting
a. rename .env.example to .env and add your private key and RPC point.

b. Start the local okc network or directly link to the test network.

## 2、Compile the wasm contract
```
$ cd project/WASMContracts

$ RUSTFLAGS='-C link-arg=-s' cargo wasm
```
You can also compile in other ways, but make sure the generated files are in the path `project/WASMContracts/target/wasm32-unknown-unknown/release/`

## 3、Deploy and test
```
$ cd project

$ npm install

$ npx hardhat test scripts/test.js
```
## principle
`evm=>cm`

After receiving the EVM specific event `__OKCSendToWasm`, the OKC will trigger a CM transaction to call the `MintCW20` method of the specified address;

`cm=>evm`

when receiving the `CosmosMsg::Custom` message sent by the CM, it will initiate a transaction through the specific address `0xc63cf6c8E1f3DF41085E9d8Af49584dae1432b4f` An EVM transaction calls the `mintERC20` method of the specified address

## notes
for `evm => cm` ,the recipient must be with "ex", for `cm => evm`, the recipient must be with "0x"