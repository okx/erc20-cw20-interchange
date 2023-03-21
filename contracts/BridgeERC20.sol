// SPDX-License-Identifier: MIT
pragma solidity ^0.8.7;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/proxy/utils/Initializable.sol";

contract BridgeERC20 is ERC20, Initializable {
    address public constant MODULE_ADDRESS =
        0xc63cf6c8E1f3DF41085E9d8Af49584dae1432b4f;
    string public wasmContractAddress;

    event Initialize(string wasmContractAddress);
    event __OKBCSendToWasm(string wasmAddr, string recipient, uint256 amount);

    constructor(
        string memory _name,
        string memory _symbol,
        uint256 _totalSupply
    ) ERC20(_name, _symbol) {
        _mint(msg.sender, _totalSupply);
    }

    function initialize(
        string calldata _wasmContractAddress
    ) public initializer {
        wasmContractAddress = _wasmContractAddress;

        emit Initialize(wasmContractAddress);
    }

    function mintERC20(
        string calldata caller,
        address recipient,
        uint256 amount
    ) external returns (bool) {
        require(
            msg.sender == MODULE_ADDRESS,
            "Only Wasm specified address can call"
        );
        require(
            keccak256(abi.encodePacked(caller)) ==
                keccak256(abi.encodePacked(wasmContractAddress)),
            "Only specified wasm contract can call"
        );
        _mint(recipient, amount);
        return true;
    }

    /**
     * @dev The function help to send erc20 token to wasm
     * @param recipient it must be "ex" address
     */
    function send_to_wasm(string memory recipient, uint128 amount) public {
        _burn(msg.sender, uint256(amount));
        emit __OKBCSendToWasm(wasmContractAddress, recipient, amount);
    }
}
