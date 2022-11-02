use cosmwasm_std::{
    entry_point, from_slice, to_binary, to_vec, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Storage, Uint128, CosmosMsg
};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};
use std::convert::TryInto;
use bech32::{self, FromBase32, ToBase32, Variant};


use crate::error::ContractError;
use crate::msg::{AllowanceResponse, BalanceResponse, ExecuteMsg, InstantiateMsg, QueryMsg,SendToEvmMsg};
use crate::state::Constants;

pub const PREFIX_CONFIG: &[u8] = b"config";
pub const PREFIX_BALANCES: &[u8] = b"balances";
pub const PREFIX_ALLOWANCES: &[u8] = b"allowances";

pub const KEY_CONSTANTS: &[u8] = b"constants";
pub const KEY_TOTAL_SUPPLY: &[u8] = b"total_supply";


#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    let total_supply: u128 = 0;

    // Check name, symbol, decimals
    if !is_valid_name(&msg.name) {
        return Err(ContractError::NameWrongFormat {});
    }
    if !is_valid_symbol(&msg.symbol) {
        return Err(ContractError::TickerWrongSymbolFormat {});
    }
    if msg.decimals > 18 {
        return Err(ContractError::DecimalsExceeded {});
    }

    let mut config_store = PrefixedStorage::new(deps.storage, PREFIX_CONFIG);
    let constants = to_vec(&Constants {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        contract: msg.evm_contract
    })?;
    config_store.set(KEY_CONSTANTS, &constants);
    config_store.set(KEY_TOTAL_SUPPLY, &total_supply.to_be_bytes());
    
    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<SendToEvmMsg>, ContractError> {
    match msg {
        ExecuteMsg::Approve { spender, amount } => try_approve(deps, env, info, spender, &amount),
        ExecuteMsg::Transfer { recipient, amount } => try_transfer(deps, env, info, recipient, &amount),
        ExecuteMsg::Burn { amount } => try_burn(deps, env, info, &amount),
        ExecuteMsg::MintCW20 { recipient, amount } => try_mint_cw20(deps, env, info, recipient, amount),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => try_transfer_from(deps, env, info, owner, recipient, &amount),
        ExecuteMsg::SendToEvm {
            recipient,
            amount,
        } => try_send_to_erc20(deps, env, info, recipient, amount),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Balance { address } => {
            let address_key = deps.api.addr_validate(&address)?;
            let balance = read_balance(deps.storage, &address_key)?;
            let out = to_binary(&BalanceResponse {
                balance: Uint128::from(balance),
            })?;
            Ok(out)
        }
        QueryMsg::Allowance { owner, spender } => {
            let owner_key = deps.api.addr_validate(&owner)?;
            let spender_key = deps.api.addr_validate(&spender)?;
            let allowance = read_allowance(deps.storage, &owner_key, &spender_key)?;
            let out = to_binary(&AllowanceResponse {
                allowance: Uint128::from(allowance),
            })?;
            Ok(out)
        }
    }
}


/**
 * 
 * @ recipient must be "ex" address,check by blockchain
 */
fn try_mint_cw20(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response<SendToEvmMsg>, ContractError> {

    //read evm contract with [u8]
    let config_storage = ReadonlyPrefixedStorage::new(deps.storage, PREFIX_CONFIG);
    let data = config_storage.get(KEY_CONSTANTS);

    let const_data: Constants = from_slice(&data.unwrap()).unwrap();
    let evm_contract_address = hex::decode(&const_data.contract[2..]).unwrap();

    //read sender address with [u8]
    let (_,data,_) = bech32::decode(&info.sender.as_str()).unwrap();
    let sender = Vec::<u8>::from_base32(&data).unwrap();

    //check tx sender is specified address
    if sender != evm_contract_address {
        return Err(ContractError::InvalidSender {
           address:info.sender.to_string()
        });
    }

    let amount_raw = amount.u128();
    //check recipient is validate
    let recipient_address = deps.api.addr_validate(recipient.as_str())?;
    let mut account_balance = read_balance(deps.storage, &recipient_address)?;

    account_balance += amount_raw;

    let mut balances_store = PrefixedStorage::new(deps.storage, PREFIX_BALANCES);
    balances_store.set(
        &recipient_address.as_str().as_bytes(),
        &account_balance.to_be_bytes(),
    );

    let mut config_store = PrefixedStorage::new(deps.storage, PREFIX_CONFIG);
    let data = config_store
        .get(KEY_TOTAL_SUPPLY)
        .expect("no total supply data stored");
    let mut total_supply = bytes_to_u128(&data).unwrap();

    total_supply += amount_raw;

    config_store.set(KEY_TOTAL_SUPPLY, &total_supply.to_be_bytes());

    Ok(Response::new()
        .add_attribute("action", "MINT")
        .add_attribute("account", recipient_address)
        .add_attribute("sender", info.sender.to_string())
        .add_attribute("amount", amount.to_string()))
}

fn try_send_to_erc20(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response<SendToEvmMsg>, ContractError> {


    //check recipient address should a ETH address
    if is_valid_eth_address(&recipient) {
        return Err(ContractError::InvalidRecipient {address: recipient});
    }

    let from = info.sender;
    let amount_raw = amount.u128();
    let mut account_balance = read_balance(deps.storage, &from)?;

    if account_balance < amount_raw {
        return Err(ContractError::InsufficientFunds {
            balance: account_balance,
            required: amount_raw,
        });
    }
    account_balance -= amount_raw;

    let mut balances_store = PrefixedStorage::new(deps.storage, PREFIX_BALANCES);
    balances_store.set(
        from.as_str().as_bytes(),
        &account_balance.to_be_bytes(),
    );

    let mut config_store = PrefixedStorage::new(deps.storage, PREFIX_CONFIG);

    //read evm contract address
    let constants_data = config_store.get(KEY_CONSTANTS);
    let const_data: Constants = from_slice(&constants_data.unwrap()).unwrap();

    //read total supply
    let total_supply_data = config_store.get(KEY_TOTAL_SUPPLY).expect("no total supply data stored");
    let mut total_supply = bytes_to_u128(&total_supply_data).unwrap();

    total_supply -= amount_raw;

    config_store.set(KEY_TOTAL_SUPPLY, &total_supply.to_be_bytes());

    //make MSG
    let message = CosmosMsg::Custom(SendToEvmMsg {
        sender: _env.contract.address.to_string(),
        contract: const_data.contract.to_string(),
        recipient,
        amount,
    });

    Ok(Response::new()
           .add_message(message)
           .add_attribute("action", "call evm")
           .add_attribute("amount", amount.to_string())
           .set_data(b"the result data"))
}

fn try_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: &Uint128,
) -> Result<Response<SendToEvmMsg>, ContractError> {
    perform_transfer(
        deps.storage,
        &info.sender,
        &deps.api.addr_validate(recipient.as_str())?,
        amount.u128(),
    )?;
    Ok(Response::new()
        .add_attribute("action", "transfer")
        .add_attribute("sender", info.sender)
        .add_attribute("recipient", recipient))
}

fn try_transfer_from(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: String,
    recipient: String,
    amount: &Uint128,
) -> Result<Response<SendToEvmMsg>, ContractError> {
    let owner_address = deps.api.addr_validate(owner.as_str())?;
    let recipient_address = deps.api.addr_validate(recipient.as_str())?;
    let amount_raw = amount.u128();

    let mut allowance = read_allowance(deps.storage, &owner_address, &info.sender)?;
    if allowance < amount_raw {
        return Err(ContractError::InsufficientAllowance {
            allowance,
            required: amount_raw,
        });
    }
    allowance -= amount_raw;
    write_allowance(deps.storage, &owner_address, &info.sender, allowance)?;
    perform_transfer(deps.storage, &owner_address, &recipient_address, amount_raw)?;

    Ok(Response::new()
        .add_attribute("action", "transfer_from")
        .add_attribute("spender", &info.sender)
        .add_attribute("sender", owner)
        .add_attribute("recipient", recipient))
}

fn try_approve(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    spender: String,
    amount: &Uint128,
) -> Result<Response<SendToEvmMsg>, ContractError> {
    let spender_address = deps.api.addr_validate(spender.as_str())?;
    write_allowance(deps.storage, &info.sender, &spender_address, amount.u128())?;
    Ok(Response::new()
        .add_attribute("action", "approve")
        .add_attribute("owner", info.sender)
        .add_attribute("spender", spender))
}

fn try_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: &Uint128,
) -> Result<Response<SendToEvmMsg>, ContractError> {
    let amount_raw = amount.u128();

    let mut account_balance = read_balance(deps.storage, &info.sender)?;

    if account_balance < amount_raw {
        return Err(ContractError::InsufficientFunds {
            balance: account_balance,
            required: amount_raw,
        });
    }
    account_balance -= amount_raw;

    let mut balances_store = PrefixedStorage::new(deps.storage, PREFIX_BALANCES);
    balances_store.set(
        info.sender.as_str().as_bytes(),
        &account_balance.to_be_bytes(),
    );

    let mut config_store = PrefixedStorage::new(deps.storage, PREFIX_CONFIG);
    let data = config_store
        .get(KEY_TOTAL_SUPPLY)
        .expect("no total supply data stored");
    let mut total_supply = bytes_to_u128(&data).unwrap();

    total_supply -= amount_raw;

    config_store.set(KEY_TOTAL_SUPPLY, &total_supply.to_be_bytes());

    Ok(Response::new()
        .add_attribute("action", "burn")
        .add_attribute("account", info.sender)
        .add_attribute("amount", amount.to_string()))
}

fn perform_transfer(
    store: &mut dyn Storage,
    from: &Addr,
    to: &Addr,
    amount: u128,
) -> Result<(), ContractError> {
    let mut balances_store = PrefixedStorage::new(store, PREFIX_BALANCES);

    let mut from_balance = match balances_store.get(from.as_str().as_bytes()) {
        Some(data) => bytes_to_u128(&data),
        None => Ok(0u128),
    }?;

    if from_balance < amount {
        return Err(ContractError::InsufficientFunds {
            balance: from_balance,
            required: amount,
        });
    }
    from_balance -= amount;
    balances_store.set(from.as_str().as_bytes(), &from_balance.to_be_bytes());

    let mut to_balance = match balances_store.get(to.as_str().as_bytes()) {
        Some(data) => bytes_to_u128(&data),
        None => Ok(0u128),
    }?;
    to_balance += amount;
    balances_store.set(to.as_str().as_bytes(), &to_balance.to_be_bytes());

    Ok(())
}

// Converts 16 bytes value into u128
// Errors if data found that is not 16 bytes
pub fn bytes_to_u128(data: &[u8]) -> Result<u128, ContractError> {
    match data[0..16].try_into() {
        Ok(bytes) => Ok(u128::from_be_bytes(bytes)),
        Err(_) => Err(ContractError::CorruptedDataFound {}),
    }
}

// Reads 16 byte storage value into u128
// Returns zero if key does not exist. Errors if data found that is not 16 bytes
pub fn read_u128(store: &ReadonlyPrefixedStorage, key: &Addr) -> Result<u128, ContractError> {
    let result = store.get(key.as_str().as_bytes());
    match result {
        Some(data) => bytes_to_u128(&data),
        None => Ok(0u128),
    }
}

fn read_balance(store: &dyn Storage, owner: &Addr) -> Result<u128, ContractError> {
    let balance_store = ReadonlyPrefixedStorage::new(store, PREFIX_BALANCES);
    read_u128(&balance_store, owner)
}

fn read_allowance(
    store: &dyn Storage,
    owner: &Addr,
    spender: &Addr,
) -> Result<u128, ContractError> {
    let owner_store =
        ReadonlyPrefixedStorage::multilevel(store, &[PREFIX_ALLOWANCES, owner.as_str().as_bytes()]);
    read_u128(&owner_store, spender)
}

#[allow(clippy::unnecessary_wraps)]
fn write_allowance(
    store: &mut dyn Storage,
    owner: &Addr,
    spender: &Addr,
    amount: u128,
) -> StdResult<()> {
    let mut owner_store =
        PrefixedStorage::multilevel(store, &[PREFIX_ALLOWANCES, owner.as_str().as_bytes()]);
    owner_store.set(spender.as_str().as_bytes(), &amount.to_be_bytes());
    Ok(())
}

fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 30 {
        return false;
    }
    true
}

fn is_valid_symbol(symbol: &str) -> bool {
    let bytes = symbol.as_bytes();
    if bytes.len() < 3 || bytes.len() > 6 {
        return false;
    }
    for byte in bytes.iter() {
        if *byte < 65 || *byte > 90 {
            return false;
        }
    }
    true
}

fn is_valid_eth_address(input: &str) -> bool {
    
    if input.len() != 42 {
        return false;
    }
    if !input.starts_with("0x") {
        return false;
    }
    true
}

fn is_valid_wasm_address(input: &str) -> bool {
    
    if input.len() != 42 {
        return false;
    }
    if !input.starts_with("ex") {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_slice, Addr, Env, MessageInfo, Storage, Timestamp, Uint128};
    use cosmwasm_storage::ReadonlyPrefixedStorage;

    fn mock_env_height(signer: &str, height: u64, time: u64) -> (Env, MessageInfo) {
        let mut env = mock_env();
        let info = mock_info(signer, &[]);
        env.block.height = height;
        env.block.time = Timestamp::from_seconds(time);
        (env, info)
    }

    fn get_constants(storage: &dyn Storage) -> Constants {
        let config_storage = ReadonlyPrefixedStorage::new(storage, PREFIX_CONFIG);
        let data = config_storage
            .get(KEY_CONSTANTS)
            .expect("no config data stored");
        from_slice(&data).expect("invalid data")
    }

    fn get_total_supply(storage: &dyn Storage) -> u128 {
        let config_storage = ReadonlyPrefixedStorage::new(storage, PREFIX_CONFIG);
        let data = config_storage
            .get(KEY_TOTAL_SUPPLY)
            .expect("no decimals data stored");
        return bytes_to_u128(&data).unwrap();
    }

    fn get_balance(storage: &dyn Storage, address: &Addr) -> u128 {
        let balances_storage = ReadonlyPrefixedStorage::new(storage, PREFIX_BALANCES);
        return read_u128(&balances_storage, address).unwrap();
    }

    fn get_allowance(storage: &dyn Storage, owner: &Addr, spender: &Addr) -> u128 {
        let owner_storage = ReadonlyPrefixedStorage::multilevel(
            storage,
            &[PREFIX_ALLOWANCES, owner.as_str().as_bytes()],
        );
        return read_u128(&owner_storage, spender).unwrap();
    }

    mod instantiate {
        use super::*;
        use crate::error::ContractError;

        #[test]
        fn works() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            };
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(
                get_constants(&deps.storage),
                Constants {
                    name: "Cash Token".to_string(),
                    symbol: "CASH".to_string(),
                    decimals: 9,
                    contract: "abc".to_string(),
                }
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
        }

        #[test]
        fn works_with_empty_balance() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            };
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(get_total_supply(&deps.storage), 0);
        }


        #[test]
        fn works_with_balance_larger_than_53_bit() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            };
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

            let mint_cw20_msg = ExecuteMsg::MintCW20 { recipient: "addr0000".to_string(), amount: (Uint128::from(9007199254740993u128)) };

            let (env, info) = mock_env_height("abc", 450, 550);
            execute(deps.as_mut(), env, info, mint_cw20_msg).unwrap();

            assert_eq!(0, res.messages.len());
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                9007199254740993
            );
            assert_eq!(get_total_supply(&deps.storage), 9007199254740993);
        }

        #[test]
        fn works_with_balance_larger_than_64_bit() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            };
            let (env, info) = mock_env_height("abc", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();

            let mint_cw20_msg = ExecuteMsg::MintCW20 { recipient: "addr0000".to_string(), amount: (Uint128::from(100000000000000000000000000u128)) };

            let (env, info) = mock_env_height("abc", 450, 550);
            execute(deps.as_mut(), env, info, mint_cw20_msg).unwrap();

            assert_eq!(0, res.messages.len());
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                100000000000000000000000000
            );
            assert_eq!(get_total_supply(&deps.storage), 100000000000000000000000000);
        }

        #[test]
        fn fails_for_large_decimals() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 42,
                evm_contract: "abc".to_string(),
            };
            let (env, info) = mock_env_height("creator", 450, 550);
            let result = instantiate(deps.as_mut(), env, info, instantiate_msg);
            match result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::DecimalsExceeded {}) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        #[test]
        fn fails_for_name_too_short() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = InstantiateMsg {
                name: "CC".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            };
            let (env, info) = mock_env_height("creator", 450, 550);
            let result = instantiate(deps.as_mut(), env, info, instantiate_msg);
            match result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::NameWrongFormat {}) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        #[test]
        fn fails_for_name_too_long() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = InstantiateMsg {
                name: "Cash coin. Cash coin. Cash coin. Cash coin.".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            };
            let (env, info) = mock_env_height("creator", 450, 550);
            let result = instantiate(deps.as_mut(), env, info, instantiate_msg);
            match result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::NameWrongFormat {}) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        #[test]
        fn fails_for_symbol_too_short() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = InstantiateMsg {
                name: "De De".to_string(),
                symbol: "DD".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            };
            let (env, info) = mock_env_height("creator", 450, 550);
            let result = instantiate(deps.as_mut(), env, info, instantiate_msg);
            match result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::TickerWrongSymbolFormat {}) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        #[test]
        fn fails_for_symbol_too_long() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = InstantiateMsg {
                name: "Super Coin".to_string(),
                symbol: "SUPERCOIN".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            };
            let (env, info) = mock_env_height("creator", 450, 550);
            let result = instantiate(deps.as_mut(), env, info, instantiate_msg);
            match result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::TickerWrongSymbolFormat {}) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        #[test]
        fn fails_for_symbol_lowercase() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CaSH".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            };
            let (env, info) = mock_env_height("creator", 450, 550);
            let result = instantiate(deps.as_mut(), env, info, instantiate_msg);
            match result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::TickerWrongSymbolFormat {}) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }
    }

    mod transfer {
        use super::*;
        use crate::error::ContractError;
        use cosmwasm_std::attr;

        fn make_instantiate_msg() -> InstantiateMsg {
            InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            }
        }

        #[test]
        fn can_send_to_existing_recipient() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addrbbbb".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
            // Transfer
            let transfer_msg = ExecuteMsg::Transfer {
                recipient: "addr1111".to_string(),
                amount: Uint128::from(0u128),
            };
            let (env, info) = mock_env_height("addr0000", 450, 550);
            let transfer_result = execute(deps.as_mut(), env, info, transfer_msg).unwrap();
            assert_eq!(transfer_result.messages.len(), 0);
            assert_eq!(
                transfer_result.attributes,
                vec![
                    attr("action", "transfer"),
                    attr("sender", "addr0000"),
                    attr("recipient", "addr1111"),
                ]
            );
            // New state
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            ); // -1
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                0
            ); // +1
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addrbbbb".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
        }

        #[test]
        fn can_send_to_non_existent_recipient() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addrbbbb".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
            // Transfer
            let transfer_msg = ExecuteMsg::Transfer {
                recipient: "addr2323".to_string(),
                amount: Uint128::from(0u128),
            };
            let (env, info) = mock_env_height("addr0000", 450, 550);
            let transfer_result = execute(deps.as_mut(), env, info, transfer_msg).unwrap();
            assert_eq!(transfer_result.messages.len(), 0);
            assert_eq!(
                transfer_result.attributes,
                vec![
                    attr("action", "transfer"),
                    attr("sender", "addr0000"),
                    attr("recipient", "addr2323"),
                ]
            );
            // New state
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr2323".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addrbbbb".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
        }

        #[test]
        fn can_send_zero_amount() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addrbbbb".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
            // Transfer
            let transfer_msg = ExecuteMsg::Transfer {
                recipient: "addr1111".to_string(),
                amount: Uint128::from(0u128),
            };
            let (env, info) = mock_env_height("addr0000", 450, 550);
            let transfer_result = execute(deps.as_mut(), env, info, transfer_msg).unwrap();
            assert_eq!(transfer_result.messages.len(), 0);
            assert_eq!(
                transfer_result.attributes,
                vec![
                    attr("action", "transfer"),
                    attr("sender", "addr0000"),
                    attr("recipient", "addr1111"),
                ]
            );
            // New state (unchanged)
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addrbbbb".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
        }

        #[test]
        fn can_send_to_sender() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let sender = "addr0000";
            // Initial state
            assert_eq!(get_balance(&deps.storage, &Addr::unchecked(sender)), 0);
            // Transfer
            let transfer_msg = ExecuteMsg::Transfer {
                recipient: sender.to_string(),
                amount: Uint128::from(0u128),
            };
            let (env, info) = mock_env_height(&sender, 450, 550);
            let transfer_result = execute(deps.as_mut(), env, info, transfer_msg).unwrap();
            assert_eq!(transfer_result.messages.len(), 0);
            assert_eq!(
                transfer_result.attributes,
                vec![
                    attr("action", "transfer"),
                    attr("sender", "addr0000"),
                    attr("recipient", "addr0000"),
                ]
            );
            // New state
            assert_eq!(get_balance(&deps.storage, &Addr::unchecked(sender)), 0);
        }

        #[test]
        fn fails_on_insufficient_balance() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addrbbbb".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
            // Transfer
            let transfer_msg = ExecuteMsg::Transfer {
                recipient: "addr1111".to_string(),
                amount: Uint128::from(12u128),
            };
            let (env, info) = mock_env_height("addr0000", 450, 550);
            let transfer_result = execute(deps.as_mut(), env, info, transfer_msg);
            match transfer_result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::InsufficientFunds {
                    balance: 0,
                    required: 12,
                }) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
            // New state (unchanged)
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addrbbbb".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
        }
    }

    mod approve {
        use super::*;
        use cosmwasm_std::attr;

        fn make_instantiate_msg() -> InstantiateMsg {
            InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            }
        }

        fn make_spender() -> Addr {
            Addr::unchecked("dadadadadadadada".to_string())
        }

        #[test]
        fn has_zero_allowance_by_default() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Existing owner
            assert_eq!(
                get_allowance(&deps.storage, &Addr::unchecked("addr0000"), &make_spender()),
                0
            );
            // Non-existing owner
            assert_eq!(
                get_allowance(
                    &deps.storage,
                    &Addr::unchecked("addr4567".to_string()),
                    &make_spender()
                ),
                0
            );
        }

        #[test]
        fn can_set_allowance() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            assert_eq!(
                get_allowance(
                    &deps.storage,
                    &Addr::unchecked("addr7654".to_string()),
                    &make_spender()
                ),
                0
            );
            // First approval
            let owner = Addr::unchecked("addr7654".to_string());
            let spender = make_spender();
            let approve_msg1 = ExecuteMsg::Approve {
                spender: spender.clone().to_string().to_string(),
                amount: Uint128::from(334422u128),
            };
            let (env, info) = mock_env_height(&owner.as_str(), 450, 550);
            let approve_result1 = execute(deps.as_mut(), env, info, approve_msg1).unwrap();
            assert_eq!(approve_result1.messages.len(), 0);
            assert_eq!(
                approve_result1.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.clone().to_string()),
                    attr("spender", spender.clone().to_string()),
                ]
            );
            assert_eq!(
                get_allowance(&deps.storage, &owner, &make_spender()),
                334422
            );
            // Updated approval
            let approve_msg = ExecuteMsg::Approve {
                spender: spender.clone().to_string().to_string(),
                amount: Uint128::from(777888u128),
            };
            let (env, info) = mock_env_height(&owner.as_str(), 450, 550);
            let approve_result2 = execute(deps.as_mut(), env, info, approve_msg).unwrap();
            assert_eq!(approve_result2.messages.len(), 0);
            assert_eq!(
                approve_result2.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.as_str()),
                    attr("spender", spender.as_str()),
                ]
            );
            assert_eq!(get_allowance(&deps.storage, &owner, &spender), 777888);
        }
    }

    mod transfer_from {
        use super::*;
        use crate::error::ContractError;
        use cosmwasm_std::{attr, Addr};

        fn make_instantiate_msg() -> InstantiateMsg {
            InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            }
        }

        fn make_spender() -> Addr {
            Addr::unchecked("dadadadadadadada".to_string())
        }

        #[test]
        fn works() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let owner = "addr0000";
            let spender = make_spender();
            let recipient = Addr::unchecked("addr1212".to_string());
            // Set approval
            let approve_msg = ExecuteMsg::Approve {
                spender: spender.clone().to_string().to_string(),
                amount: Uint128::from(4u128),
            };
            let (env, info) = mock_env_height(&owner.clone(), 450, 550);
            let approve_result = execute(deps.as_mut(), env, info, approve_msg).unwrap();
            assert_eq!(approve_result.messages.len(), 0);
            assert_eq!(
                approve_result.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.clone().to_string()),
                    attr("spender", spender.clone().to_string()),
                ]
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked(owner.clone())),
                0
            );
            assert_eq!(
                get_allowance(&deps.storage, &Addr::unchecked(owner.clone()), &spender),
                4
            );
            // Transfer less than allowance but more than balance
            let transfer_from_msg = ExecuteMsg::TransferFrom {
                owner: owner.clone().to_string().to_string(),
                recipient: recipient.clone().to_string(),
                amount: Uint128::from(0u128),
            };
            let (env, info) = mock_env_height(&spender.as_str(), 450, 550);
            let transfer_from_result =
                execute(deps.as_mut(), env, info, transfer_from_msg).unwrap();
            assert_eq!(transfer_from_result.messages.len(), 0);
            assert_eq!(
                transfer_from_result.attributes,
                vec![
                    attr("action", "transfer_from"),
                    attr("spender", spender.clone()),
                    attr("sender", owner),
                    attr("recipient", recipient),
                ]
            );
            // State changed
            assert_eq!(get_balance(&deps.storage, &Addr::unchecked(owner)), 0);
            assert_eq!(
                get_allowance(&deps.storage, &Addr::unchecked(owner), &spender),
                4
            );
        }

        #[test]
        fn fails_when_allowance_too_low() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let owner = "addr0000";
            let spender = make_spender();
            let recipient = Addr::unchecked("addr1212".to_string());
            // Set approval
            let approve_msg = ExecuteMsg::Approve {
                spender: spender.clone().to_string(),
                amount: Uint128::from(2u128),
            };
            let (env, info) = mock_env_height(&owner.clone(), 450, 550);
            let approve_result = execute(deps.as_mut(), env, info, approve_msg).unwrap();
            assert_eq!(approve_result.messages.len(), 0);
            assert_eq!(
                approve_result.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.clone().to_string()),
                    attr("spender", spender.clone().to_string()),
                ]
            );
            assert_eq!(get_balance(&deps.storage, &Addr::unchecked(owner)), 0);
            assert_eq!(
                get_allowance(&deps.storage, &Addr::unchecked(owner), &spender),
                2
            );
            // Transfer less than allowance but more than balance
            let fransfer_from_msg = ExecuteMsg::TransferFrom {
                owner: owner.clone().to_string(),
                recipient: recipient.clone().to_string(),
                amount: Uint128::from(3u128),
            };
            let (env, info) = mock_env_height(&spender.as_str(), 450, 550);
            let transfer_result = execute(deps.as_mut(), env, info, fransfer_from_msg);
            match transfer_result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::InsufficientAllowance {
                    allowance: 2,
                    required: 3,
                }) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        #[test]
        fn fails_when_allowance_is_set_but_balance_too_low() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let owner = "addr0000";
            let spender = make_spender();
            let recipient = Addr::unchecked("addr1212".to_string());
            // Set approval
            let approve_msg = ExecuteMsg::Approve {
                spender: spender.clone().to_string(),
                amount: Uint128::from(100u128),
            };
            let (env, info) = mock_env_height(&owner.clone(), 450, 550);
            let approve_result = execute(deps.as_mut(), env, info, approve_msg).unwrap();
            assert_eq!(approve_result.messages.len(), 0);
            assert_eq!(
                approve_result.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.clone().to_string()),
                    attr("spender", spender.clone().to_string()),
                ]
            );
            assert_eq!(get_balance(&deps.storage, &Addr::unchecked(owner)), 0);
            assert_eq!(
                get_allowance(&deps.storage, &Addr::unchecked(owner), &spender),
                100
            );
            // Transfer less than allowance but more than balance
            let fransfer_from_msg = ExecuteMsg::TransferFrom {
                owner: owner.clone().to_string(),
                recipient: recipient.clone().to_string(),
                amount: Uint128::from(100u128),
            };
            let (env, info) = mock_env_height(&spender.as_str(), 450, 550);
            let transfer_result = execute(deps.as_mut(), env, info, fransfer_from_msg);
            match transfer_result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::InsufficientFunds {
                    balance: 0,
                    required: 100,
                }) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }
    }

    mod burn {
        use super::*;
        use crate::error::ContractError;
        use cosmwasm_std::{attr, Addr};

        fn make_instantiate_msg() -> InstantiateMsg {
            InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            }
        }

        #[test]
        fn can_burn_zero_amount() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
            // Burn
            let burn_msg = ExecuteMsg::Burn {
                amount: Uint128::from(0u128),
            };
            let (env, info) = mock_env_height("addr0000", 450, 550);
            let burn_result = execute(deps.as_mut(), env, info, burn_msg).unwrap();
            assert_eq!(burn_result.messages.len(), 0);
            assert_eq!(
                burn_result.attributes,
                vec![
                    attr("action", "burn"),
                    attr("account", "addr0000"),
                    attr("amount", "0"),
                ]
            );
            // New state (unchanged)
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
        }

        #[test]
        fn fails_on_insufficient_balance() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("creator", 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            // Initial state
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
            // Burn
            let burn_msg = ExecuteMsg::Burn {
                amount: Uint128::from(12u128),
            };
            let (env, info) = mock_env_height("addr0000", 450, 550);
            let burn_result = execute(deps.as_mut(), env, info, burn_msg);
            match burn_result {
                Ok(_) => panic!("expected error"),
                Err(ContractError::InsufficientFunds {
                    balance: 0,
                    required: 12,
                }) => {}
                Err(e) => panic!("unexpected error: {:?}", e),
            }
            // New state (unchanged)
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr0000".to_string())),
                0
            );
            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                0
            );
            assert_eq!(get_total_supply(&deps.storage), 0);
        }
    }

    mod query {
        use super::*;
        use cosmwasm_std::{attr, Addr};

        fn address(index: u8) -> Addr {
            match index {
                0 => Addr::unchecked("addr0000".to_string()), // contract instantiateializer
                1 => Addr::unchecked("addr1111".to_string()),
                2 => Addr::unchecked("addr4321".to_string()),
                3 => Addr::unchecked("addr5432".to_string()),
                4 => Addr::unchecked("addr6543".to_string()),
                _ => panic!("Unsupported address index"),
            }
        }

        fn make_instantiate_msg() -> InstantiateMsg {
            InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                evm_contract: "abc".to_string(),
            }
        }

        #[test]
        fn can_query_balance_of_existing_address() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height(&address(0).as_str(), 450, 550);
            let res = instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let query_msg = QueryMsg::Balance {
                address: address(1).to_string(),
            };
            let query_result = query(deps.as_ref(), env, query_msg).unwrap();
            assert_eq!(query_result.as_slice(), b"{\"balance\":\"0\"}");
        }

        #[test]
        fn can_query_balance_of_nonexisting_address() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height(&address(0).as_str(), 450, 550);
            let res = instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let query_msg = QueryMsg::Balance {
                address: address(4).to_string(), // only indices 1, 2, 3 are instantiateialized
            };
            let query_result = query(deps.as_ref(), env, query_msg).unwrap();
            assert_eq!(query_result.as_slice(), b"{\"balance\":\"0\"}");
        }

        #[test]
        fn can_query_allowance_of_existing_addresses() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height(&address(0).as_str(), 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let owner = address(2);
            let spender = address(1);
            let approve_msg = ExecuteMsg::Approve {
                spender: spender.clone().to_string(),
                amount: Uint128::from(42u128),
            };
            let (env, info) = mock_env_height(&owner.as_str(), 450, 550);
            let action_result = execute(deps.as_mut(), env.clone(), info, approve_msg).unwrap();
            assert_eq!(action_result.messages.len(), 0);
            assert_eq!(
                action_result.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.clone().to_string()),
                    attr("spender", spender.clone().to_string()),
                ]
            );
            let query_msg = QueryMsg::Allowance {
                owner: owner.clone().to_string(),
                spender: spender.clone().to_string(),
            };
            let query_result = query(deps.as_ref(), env.clone(), query_msg).unwrap();
            assert_eq!(query_result.as_slice(), b"{\"allowance\":\"42\"}");
        }

        #[test]
        fn can_query_allowance_of_nonexisting_owner() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height(&address(0).as_str(), 450, 550);
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());
            let owner = address(2);
            let spender = address(1);
            let bob = address(3);
            let approve_msg = ExecuteMsg::Approve {
                spender: spender.clone().to_string(),
                amount: Uint128::from(42u128),
            };
            let (env, info) = mock_env_height(&owner.as_str(), 450, 550);
            let approve_result = execute(deps.as_mut(), env.clone(), info, approve_msg).unwrap();
            assert_eq!(approve_result.messages.len(), 0);
            assert_eq!(
                approve_result.attributes,
                vec![
                    attr("action", "approve"),
                    attr("owner", owner.clone().to_string()),
                    attr("spender", spender.clone().to_string()),
                ]
            );
            // different spender
            let query_msg = QueryMsg::Allowance {
                owner: owner.clone().to_string(),
                spender: bob.clone().to_string(),
            };
            let query_result = query(deps.as_ref(), env.clone(), query_msg).unwrap();
            assert_eq!(query_result.as_slice(), b"{\"allowance\":\"0\"}");
            // differnet owner
            let query_msg = QueryMsg::Allowance {
                owner: bob.clone().to_string(),
                spender: spender.clone().to_string(),
            };
            let query_result = query(deps.as_ref(), env.clone(), query_msg).unwrap();
            assert_eq!(query_result.as_slice(), b"{\"allowance\":\"0\"}");
        }
    }
    
    // mod bridge {
    //     use super::*;
    //     use cosmwasm_std::{attr, Addr};

    //     fn address(index: u8) -> Addr {
    //         match index {
    //             0 => Addr::unchecked("addr0000".to_string()), // contract instantiateializer
    //             1 => Addr::unchecked("addr1111".to_string()),
    //             2 => Addr::unchecked("addr4321".to_string()),
    //             3 => Addr::unchecked("addr5432".to_string()),
    //             4 => Addr::unchecked("addr6543".to_string()),
    //             _ => panic!("Unsupported address index"),
    //         }
    //     }

    //     fn make_instantiate_msg() -> InstantiateMsg {
    //         InstantiateMsg {
    //             name: "Cash Token".to_string(),
    //             symbol: "CASH".to_string(),
    //             decimals: 9,
    //             evm_contract: "0xcd38B80aee05cad65571B7564BD110fdf2990de6".to_string(),
    //         }
    //     }

    //     #[test]
    //     fn can_send_to_wasm() {
    //         let mut deps = mock_dependencies(&[]);
    //         let instantiate_msg = make_instantiate_msg();
    //         let (env, info) = mock_env_height("0xcd38B80aee05cad65571B7564BD110fdf2990de6", 450, 550);
    //         let res = instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();
    //         assert_eq!(0, res.messages.len());

    //         let mint_cw20_msg = ExecuteMsg::MintCW20 { recipient: "0xcd38B80aee05cad65571B7564BD110fdf2990de6".to_string(), amount: (Uint128::from(100u128)) };

    //         let (env, info) = mock_env_height("ex1e5utszhwqh9dv4t3katyh5gslhefjr0xmlcyyr", 450, 550);
    //         let mint_cw20_result = execute(deps.as_mut(), env, info, mint_cw20_msg).unwrap();
    //         assert_eq!(mint_cw20_result.messages.len(), 0);
    //         assert_eq!(
    //             mint_cw20_result.attributes,
    //             vec![
    //                 attr("action", "MINT"),
    //                 attr("account", "addr1111"),
    //                 attr("sender", "addr0000"),
    //                 attr("amount","100"),
    //             ]
    //         );

    //         assert_eq!(
    //             get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
    //             100
    //         );
    //         assert_eq!(get_total_supply(&deps.storage), 100);
    //     } 

    //     #[test]
    //     fn can_send_to_evm(){
    //         let mut deps = mock_dependencies(&[]);
    //         let instantiate_msg = make_instantiate_msg();
    //         let (env, info) = mock_env_height(&address(0).as_str(), 450, 550);
    //         let res = instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();
    //         assert_eq!(0, res.messages.len());

    //         let mint_cw20_msg = ExecuteMsg::MintCW20 { recipient: address(1).to_string(), amount: (Uint128::from(100u128)) };

    //         let (env, info) = mock_env_height("addr0000", 450, 550);
    //         execute(deps.as_mut(), env, info, mint_cw20_msg).unwrap();

    //         let send_to_evm_msg = ExecuteMsg::SendToEvm {recipient: address(1).to_string(), amount: (Uint128::from(100u128)) };

    //         let (env, info) = mock_env_height("addr1111", 450, 550);
    //         let send_to_evm_result = execute(deps.as_mut(), env, info, send_to_evm_msg).unwrap();

    //         print!("这是一个打印结果{:?}",send_to_evm_result);
    //         assert_eq!(send_to_evm_result.messages.len(), 1);
    //         assert_eq!(send_to_evm_result.attributes, 
    //             vec![
    //                 attr("action","call evm"),
    //                 attr("amount","100")
    //             ]);

    //         assert_eq!(
    //             get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
    //             0
    //         );
    //     }
    // }


    mod deploy_test {
        use super::*;
        use cosmwasm_std::{attr, Addr};

        fn address(index: u8) -> Addr {
            match index {
                0 => Addr::unchecked("addr0000".to_string()), // contract instantiateializer
                1 => Addr::unchecked("addr1111".to_string()),
                2 => Addr::unchecked("addr4321".to_string()),
                3 => Addr::unchecked("addr5432".to_string()),
                4 => Addr::unchecked("addr6543".to_string()),
                _ => panic!("Unsupported address index"),
            }
        }

        fn make_instantiate_msg() -> InstantiateMsg {
            InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                evm_contract: "0xcd38b80aee05cad65571b7564bd110fdf2990de6".to_string(),
            }
        }

        #[test]
        fn contract_can_same() {
            let mut deps = mock_dependencies(&[]);
            let instantiate_msg = make_instantiate_msg();
            let (env, info) = mock_env_height("ex1e5utszhwqh9dv4t3katyh5gslhefjr0xmlcyyr", 450, 550);
            let res = instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());

            let mint_cw20_msg = ExecuteMsg::MintCW20 { recipient: "addr111".to_string(), amount: (Uint128::from(100u128)) };
            
            let (env, info) = mock_env_height("ex1e5utszhwqh9dv4t3katyh5gslhefjr0xmlcyyr", 450, 550);
            let mint_cw20_result = execute(deps.as_mut(), env, info, mint_cw20_msg).unwrap();
            assert_eq!(mint_cw20_result.messages.len(), 0);
            assert_eq!(
                mint_cw20_result.attributes,
                vec![
                    attr("action", "MINT"),
                    attr("account", "addr111"),
                    attr("sender", "ex1e5utszhwqh9dv4t3katyh5gslhefjr0xmlcyyr"),
                    attr("amount","100"),
                ]
            );

            assert_eq!(
                get_balance(&deps.storage, &Addr::unchecked("addr1111".to_string())),
                100
            );
            assert_eq!(get_total_supply(&deps.storage), 100);
        } 
    }

}