// SPDX-License-Identifier: MIT

pragma solidity 0.8.7;

import "./ERC20.sol";

contract VMTokenBridge {

    address internal constant moduleAddress = 0xc63cf6c8E1f3DF41085E9d8Af49584dae1432b4f;
    string internal  immutable wasmContractAddress;
    address internal immutable evmContractAddress;

    event Initialize(string wasmContractAddress, address evmContractAddress);
    event __OKCSendToWasm(string wasmAddr, string recipient, uint256 amount);

    modifier onlyWasm() {
        require(msg.sender == moduleAddress, "Only Wasm specified address can call");
    }

    modifier onlyWasmContract(string calldata caller){
        require(caller == wasmContractAddress, "Only specified wasm contract can call");
    }

    function initialize(
        string calldata _wasmContractAddress,
        address _evmContractAddress
    
    ) public {

        wasmContractAddress = _wasmContractAddress;
        evmContractAddress = _evmContractAddress;

        emit Initialize(wasmContractAddress, evmContractAddress);
    }

    function mintERC20(
        string calldata caller, 
        address recipient,
        uint256 amount
    ) 
    external override 
    onlyWasm 
    onlyWasmContract(caller) 
    returns (bool) 
    {
        IERC20(evmContractAddress).transfer(recipient, amount);
        return true;
    }

    function send_to_wasm(string memory recipient,string memory wasmContract , uint256 amount) public {
        
        IERC20(evmContractAddress).transferFrom(msg.sender,address(this),amount);

        emit __OKCSendToWasm(wasmContract,recipient, amount);
    }
}