var encoding =require("@cosmjs/encoding");
var amino = require("@cosmjs/amino");
var wasm = require("cosmwasm");
const hre = require("hardhat");
const { expect } = require("chai");
require('dotenv').config();

var wasmDeploy = require("./WASMdeploy");
var toolbox = require("./tools");

describe("WASM bridge", function () {
  
  let wasmContract  
  let bridgeERC20

  let exBridgeERC20Address
  let wasmClient

  let aliceInEvm, aliceInWASM


  before( async() => {
    //get test user
    //get test from hardhat
    [aliceInEvm,] = await hre.ethers.getSigners();
    let signer = await amino.Secp256k1Wallet.fromKey(encoding.fromHex(process.env.TEST_USER1_PRIVATE_KEY),"ex");
    [aliceInWASM,] = await signer.getAccounts();

    //Wallet connect WASM
    wasmClient = await wasm.SigningCosmWasmClient.connectWithSigner(process.env.RPC_END_POINT,signer);

    //show address
    console.log("aliceInEvm is "+aliceInEvm.address)
    console.log("aliceInWASM is "+aliceInWASM.address)

    //deploy evm contract by hardhat
    const BridgeERC20 = await hre.ethers.getContractFactory("BridgeERC20");
    bridgeERC20 = await BridgeERC20.connect(aliceInEvm).deploy("testERC20","T20", 10000);
    await bridgeERC20.deployed()
    //show EVM contract
    console.log("EVM contract is "+bridgeERC20.address);

    //deploy WASM contract(include init)
    wasmContract =  await wasmDeploy.deployWASMContract(bridgeERC20.address);
    console.log("WASM contract is "+ wasmContract.contractAddress)
    console.log("init WASM contract ok!")

    //init EVM contract
    let result = await bridgeERC20.connect(aliceInEvm).initialize(wasmContract.contractAddress);
    await result.wait();
    console.log("init EVM contract ok!")
  })


  it("erc20 =>cw20 should success", async () => {


      result = await wasmClient.queryContractSmart(wasmContract.contractAddress, { balance: {address:aliceInWASM.address} });
      expect(result.balance, 0)
      result = await bridgeERC20.connect(aliceInEvm).send_to_wasm(aliceInWASM.address, 1000);
      //wait tx
      let txReceipt = await result.wait();
      expect(txReceipt.status, 1);
      
      result = await bridgeERC20.connect(aliceInEvm).balanceOf(aliceInEvm.address)
      expect(result, 9000)
      result = await wasmClient.queryContractSmart(wasmContract.contractAddress, { balance: {address:aliceInWASM.address} });
      expect(result.balance, 1000)
  })

  it("cw20 => erc20 should success",async() =>{

      result = await bridgeERC20.connect(aliceInEvm).balanceOf(aliceInEvm.address)
      expect(result, 9000)
      result = await wasmClient.queryContractSmart(wasmContract.contractAddress, { balance: {address:aliceInWASM.address} });
      expect(result.balance, 1000)
  
      let wasmResult = await wasmClient.execute(aliceInWASM.address,wasmContract.contractAddress,{"send_to_evm":{"recipient":aliceInEvm.address, "amount":"1000"}},{"amount":wasm.parseCoins("200000000000000000wei"),"gas":"20000000"})

      result = await bridgeERC20.connect(aliceInEvm).balanceOf(aliceInEvm.address)
      expect(result, 10000)
      result = await wasmClient.queryContractSmart(wasmContract.contractAddress, { balance: {address:aliceInWASM.address} });
      expect(result.balance, 0)
  })

  it("erc20 => cw20 with too big amount should fail", async() =>{
      await expect(
        bridgeERC20.connect(aliceInEvm).send_to_wasm(aliceInWASM.address, 100000)
      ).to.be.revertedWith("ERC20: burn amount exceeds balance")
  })

  it("erc20 => cw20 with error address should fail", async() =>{
    let errorUser = "thisISAErrorAddress"
    let wasmTxErr

    result = await bridgeERC20.connect(aliceInEvm).send_to_wasm(errorUser, 100)
    try{
      await result.wait();
    }catch(err){
      wasmTxErr = err
    }

    expect(wasmTxErr.toString()).to.contain("Error: transaction failed")
    result = await bridgeERC20.connect(aliceInEvm).balanceOf(aliceInEvm.address)
    expect(
      result
    ).equal(10000)
  })

  it("erc20 => cw20 with evm address should fail", async() =>{
    let wasmTxErr

    result = await bridgeERC20.connect(aliceInEvm).send_to_wasm(aliceInEvm.address, 100)
    try{
       await result.wait()
    }catch(err){
      wasmTxErr = err
    }

    expect(wasmTxErr.toString()).to.contain("Error: transaction failed")
    result = await bridgeERC20.connect(aliceInEvm).balanceOf(aliceInEvm.address)
    expect(
      result
    ).equal(10000)
  })

  it("erc20 => cw20 call mintERC20 by user should fail", async() =>{
    await expect(
    bridgeERC20.connect(aliceInEvm).mintERC20(aliceInEvm.address,aliceInEvm.address, 1000)
    ).to.be.revertedWith("Only Wasm specified address can call")
  })

  it("cw20 => erc20 with error address should fail", async() =>{

    let errorUser = "thisISAErrorAddress"
    let wasmTxErr
    await bridgeERC20.connect(aliceInEvm).send_to_wasm(aliceInWASM.address, 1000);

    try{
      result = await wasmClient.execute(aliceInWASM.address,wasmContract.contractAddress,{"send_to_evm":{"recipient":errorUser.address, "amount":"10"}},{"amount":wasm.parseCoins("200000000000000000wei"),"gas":"20000000"})
      // await result.wait(1);
      // console.log("这是返回值"+result.code)
      console.log(result)
    }catch(err){
      wasmTxErr = err
    }
    expect(wasmTxErr.toString()).to.contain("cw_erc20::msg::ExecuteMsg: missing field `recipient`")
    
  })

  it("cw20 => erc20 with too big amount should fail", async() =>{

    let wasmTxErr
    try {
      await wasmClient.execute(aliceInWASM.address,wasmContract.contractAddress,{"send_to_evm":{"recipient":aliceInEvm.address, "amount":"2000"}},{"amount":wasm.parseCoins("200000000000000000wei"),"gas":"20000000"})
    }
    catch(err){
      wasmTxErr = err
    }

    expect(wasmTxErr.toString()).to.contain("execute wasm contract failed: Insufficient funds (balance 1000, required=2000)")
    
  })

  it("erc20 => cw20 call mintERC20 by user should fail", async() =>{

    let wasmTxErr
    try {
      await wasmClient.execute(aliceInWASM.address,wasmContract.contractAddress,{"mint_c_w20":{"recipient":aliceInEvm.address, "amount":"2000"}},{"amount":wasm.parseCoins("200000000000000000wei"),"gas":"20000000"})
    }
    catch(err){
      wasmTxErr =err
    }

    expect(wasmTxErr.toString()).to.contain("The sender addr "+aliceInWASM.address+" is not expect")
  })

})
