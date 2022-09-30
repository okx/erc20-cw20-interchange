// SPDX-License-Identifier: MIT
pragma solidity ^0.8.7;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";

contract Bridge is Initializable{

    address internal constant MODULE_ADDRESS = 0xc63cf6c8E1f3DF41085E9d8Af49584dae1432b4f;
    string internal wasmContractAddress;
    address internal evmContractAddress;

    event Initialize(string wasmContractAddress, address evmContractAddress);
    event __OKCSendToWasm(string wasmAddr, string recipient, uint256 amount);

    modifier onlyWasm() {
        require(msg.sender == MODULE_ADDRESS, "Only Wasm specified address can call");
        _;
    }

    modifier onlyWasmContract(string memory caller){
        require(keccak256(abi.encodePacked(caller)) == keccak256(abi.encodePacked(wasmContractAddress)), "Only specified wasm contract can call");
        _;
    }

    function initialize(
        string calldata _wasmContractAddress,
        address _token
    
    ) public initializer{

        wasmContractAddress = _wasmContractAddress;
        evmContractAddress = _token;

        emit Initialize(wasmContractAddress, evmContractAddress);
    }

    function mintERC20(
        string calldata caller, 
        address recipient,
        uint256 amount
    ) 
    external 
    onlyWasm 
    onlyWasmContract(caller) 
    returns (bool) 
    {
        IERC20(evmContractAddress).transfer(recipient, amount);
        return true;
    }

    function send_to_wasm(string memory recipient, string memory wasmContract, uint256 amount) public {
        
        IERC20(evmContractAddress).transferFrom(msg.sender, address(this), amount);

        emit __OKCSendToWasm(wasmContract,recipient, amount);
    }
}