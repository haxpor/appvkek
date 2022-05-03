use web3::{
    Web3,
    types::{Address, U256},
    transports::http::Http,
    contract::{Contract, Options, tokens::Detokenize},
};
use regex::Regex;
use ::evmscan::prelude::*;

pub type Web3Type = web3::Web3<web3::transports::http::Http>;

/// RPC endpoint of BSC chain
pub(crate) static BSC_RPC_ENDPOINT: &str = "https://bsc-dataseed.binance.org/";
/// RPC endpoint of Ethereum chain
pub(crate) static ETHEREUM_RPC_ENDPOINT: &str = "https://rpc.ankr.com/eth";
/// RPC endpoint of Polygon chain
pub(crate) static POLYGON_RPC_ENDPOINT: &str = "https://polygon-rpc.com/";

/// Validate whether the specified address is in correct format.
/// Return true if the format is correct, otherwise return false.
///
/// # Arguments
/// * `address` - address to check its format correctness
pub fn validate_address_format(address: &str) -> bool {
    let lowercase_address = address.to_lowercase();
    let regex: Regex = Regex::new(r#"^(0x)?[0-9a-f]{40}$"#).unwrap();

    regex.is_match(&lowercase_address)
}

/// Perform check whether the specified address is an EOA.
/// Return true if it is, otherwise return false.
///
/// # Arguments
/// * `web3` - instance of web3
/// * `address` - address to check; in format `0x...`.
pub async fn perform_check_is_eoa(web3: &Web3<Http>, address: &str) -> Result<bool, String> {
    if !validate_address_format(address) {
        return Err(format!("Error address is not in the correct format; addr={}", address));
    }

    // convert into hex bytes in order to create `web3::Address`
    let address_hexbytes_decoded = match hex::decode(&address[2..]) {
        Ok(res) => res,
        Err(e) => {
            let err_msg = format!("Error hex decoding of address ({}); err={}", address, e);
            return Err(err_msg);
        }
    };
    
    // query for code
    let code_bytes = match web3.eth().code(Address::from_slice(address_hexbytes_decoded.as_slice()), None).await {
        Ok(res) => res,
        Err(e) => {
            let err_msg = format!("Error awaiting result for code from address ({}); err={}", address, e);
            return Err(err_msg);
        }
    };

    // encode hex bytes into hex string
    let code_str = hex::encode(code_bytes.0.as_slice());

    if code_str.len() > 0 {
        // it is a contract address
        return Ok(false);
    }

    Ok(true)
}

/// Get `Address` from string literal.
///
/// # Arguments
/// * `address` - address string literal prefixed with '0x'
pub fn get_address_from_str(address: &str) -> Result<Address, String> {
    if !validate_address_format(address) {
        return Err(format!("Error address is not in the correct format; addr={}", address));
    }
    
    Ok(Address::from_slice(hex::decode(&address[2..]).unwrap().as_slice()))
}

/// Create a web3 instance
pub fn create_web3(chain: ChainType) -> Web3<Http> {
    let rpc_endpoint = match chain {
        ChainType::BSC => BSC_RPC_ENDPOINT,
        ChainType::Ethereum => ETHEREUM_RPC_ENDPOINT,
        ChainType::Polygon => POLYGON_RPC_ENDPOINT,
    };
    let http = Http::new(rpc_endpoint).unwrap();
    Web3::new(http)
}

/// Parse a long hex string into vector of hex string of 64 characters in length (256 bit)
/// excluding the prefixed method-id which has 8 characters in length (32 bit).
/// Return a vector of hex string of 64 characters in length (256 bit);
///
/// # Arguments
/// * `long_hex_str` - input long hex string to parse; included a prefix of `0x`
pub fn parse_256_method_arguments(long_hex_str: &str) -> Result<Vec<String>, String> {
    if long_hex_str.len() == 0 {
        return Ok(Vec::new());
    }

    // get slice excluding prefix of method-id
    let arguments_hex_str = &long_hex_str[10..];

    // the length of input stringis not long enough to get at least one element
    if arguments_hex_str.len() < 64 {
        return Err("Input hex string length is not long enough to be parsed.
It needs to have at least 64 characters in length included with prefix of 0x".to_owned());
    }

    let mut offset_i: usize = 0;
    let mut res_vec: Vec<String> = Vec::new();

    while offset_i + 64 <= arguments_hex_str.len() {
        res_vec.push((&arguments_hex_str[offset_i..offset_i+64]).to_owned());
        offset_i = offset_i + 64;
    }

    Ok(res_vec)
}

/// Create a contract
///
/// # Arguments
/// * `web3` - web3 instance
/// * `contract_address_str` - contract address string
/// * `abi_str` - abi
pub fn create_contract(web3: &Web3<Http>, contract_address_str: &str, abi_str: &str) -> Result<Contract<Http>, String> {
    if !validate_address_format(contract_address_str) {
        let err_msg = format!("Error address is in wrong format ({}).", contract_address_str);
        return Err(err_msg);
    }
    let contract_address_hbytes = match hex::decode(&contract_address_str[2..]) {
        Ok(res) => res,
        Err(e) => return Err(format!("Error converting from literal string of contract address into hex bytes; err={}", e)),
    };
    let contract_address: Address = Address::from_slice(contract_address_hbytes.as_slice());

    // create a contract from contract address, and abi
    match Contract::from_json(web3.eth(), contract_address, abi_str.as_bytes()) {
        Ok(res) => Ok(res),
        Err(e) => {
            let err_msg = format!("Error creating contract associated with abi for {}; err={}", contract_address_str, e);
            Err(err_msg)
        }
    }
}

/// Query for allowanced balance.
///
/// # Arguments
/// * `contract` - `web3::contract::Contract` instance
/// * `owner_address_str` - literal string of owner address (prefixed with '0x') that permits
/// spender address to spend
/// * `spender_address_str` - literawl string of spender address (prefixed with '0x') that will
/// spend tokens on behalf of owner
pub async fn query_allowance_balance(contract: &Contract<Http>, owner_address_str: &str, spender_address_str: &str) -> Result<U256, String> {
    // NOTE: assume input `contract_address_str` is actually a contract address
    // without check.

    // validate the address format for all address inputs
    if !validate_address_format(owner_address_str) {
        let err_msg = format!("Error address is in wrong format ({}).", owner_address_str);
        return Err(err_msg);
    }
    if !validate_address_format(spender_address_str) {
        let err_msg = format!("Error address is in wrong format ({}).", spender_address_str);
        return Err(err_msg);
    }

    let owner_address = get_address_from_str(owner_address_str)?;
    let spender_address = get_address_from_str(spender_address_str)?;

    // make query for allowanceuu
    let allowance_res = contract.query("allowance", (owner_address, spender_address,), None, Options::default(), None).await;

    match allowance_res {
        Ok(allowance) => Ok(allowance),
        Err(e) => Err(format!("Error querying via RPC for allowance; owner addr={}, spender addr={}; err={}", owner_address_str, spender_address_str, e)),
    }
}

// NOTE: Interesting hidden type captures the anonymous lifetime
/// Utility function to make a web3 query.
/// Internally this function will use default options with no parameters specified
/// to make call to specified function.
///
/// This requires no gas fee as it is a query which changes no states of blockchain.
///
/// # Arguments
/// * `contract` - `web3::contract::Contract`
/// * `fn_name` - name of function to make a call
pub fn web3_query_no_params<'a, R>(contract: &'a Contract<Http>, fn_name: &'a str) -> impl core::future::Future<Output = web3::contract::Result<R>> + 'a
where
    R: Detokenize + 'a
{
    contract.query(fn_name, (), None, Options::default(), None)
}

/// Start measuring time. Suitable for wall-clock time measurement.
/// This is mainly used to measure time of placing a limit order onto Bybit.
pub fn measure_start(start: &mut std::time::Instant) {
    *start = std::time::Instant::now();
}

/// Mark the end of the measurement of time performance.
/// Return result in seconds, along with printing the elapsed time if `also_print`
/// is `true`.
pub fn measure_end(start: &std::time::Instant, also_print: bool) -> f64 {
    let elapsed = start.elapsed().as_secs_f64();
    if also_print {
        println!("(elapsed = {:.2} secs)", elapsed);
    }
    elapsed
}
