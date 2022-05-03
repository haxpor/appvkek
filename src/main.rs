use ::evmscan::evmscan;
use ::evmscan::environ::Context;
use ::evmscan::prelude::*;
use ::evmscan::prelude::U256 as BSCU256;    // for floating-point representation for U256
use clap::Parser;
use std::collections::HashMap;

mod types;
mod util;

use types::*;
use util::*;

// to avoid having to relying on reading external file
// currently contains "name", "decimals", "allowance", and "approve" (this one is not used yet)
static ABI_STR: &'static str = r#"[{"inputs":[],"name":"name","outputs":[{"internalType":"string","name":"","type":"string"}],"stateMutability":"view","type":"function"},{"inputs":[],"name":"decimals","outputs":[{"internalType":"uint8","name":"","type":"uint8"}],"stateMutability":"view","type":"function"},{"name":"allowance","inputs":[{"internalType":"address","name":"owner","type":"address"},{"internalType":"address","name":"spender","type":"address"}],"outputs":[{"internalType":"uint256","name":"","type":"uint256"}],"stateMutability":"view","type":"function"},{"name":"approve","inputs":[{"internalType":"address","name":"spender","type":"address"},{"internalType":"uint256","name":"amount","type":"uint256"}],"outputs":[{"internalType":"bool","name":"","type":"bool"}],"stateMutability":"nonpayable","type":"function"}]"#;

/// Make query for information towards token contract address, and associated
/// spender addresses with their allowance balances.
///
/// Return `TokenContractWithSpenderAllowances` structure, otherwise return
/// tuple of `(token_contract_address, error_message)`.
///
/// # Note
/// As the query needs to live long enough, thus its function's arguments need
/// to live long enough as well e.g. address is in `String` not `&str`.
///
/// # Arguments
/// * `web3` - web3 instance
/// * `contract_address` - token contract address to interact with
/// * `owner_address` - owner wallet address
/// * `spenders` - all spender addresses associated with such token contract address
async fn query(web3: &Web3Type, contract_address: String, owner_address: String, spenders: Vec<String>) -> Result<TokenContractWithSpenderAllowances, (String, String)> {
    let contract = match create_contract(&web3, &contract_address, &ABI_STR) {
        Ok(res) => res,
        Err(e) => {
            let err_msg = format!("{}", e);
            return Err( (contract_address, err_msg) );
        }
    };

    // 1. multiple top-level queries starting from here...
    let name_f = web3_query_no_params::<String>(&contract, "name");
    let decimals_f = web3_query_no_params::<u8>(&contract, "decimals");

    let (name, decimals) = futures::join!(name_f, decimals_f);

    if name.is_err() {
        let err_msg = format!("Error in querying top-level query (name); err={}", name.unwrap_err());
        return Err( (contract_address, err_msg) );
    }
    if decimals.is_err() {
        let err_msg = format!("Error in querying top-level query (decimals); err={}", decimals.unwrap_err());
        return Err( (contract_address, err_msg) );
    }

    let mut result_struct = TokenContractWithSpenderAllowances {
        name: name.unwrap(),
        address: contract_address.to_owned(),
        decimals: decimals.unwrap(),
        spender_allowances: HashMap::new(),
    };

    // 2. spender' allowances
    // make query to get current allowanced balance
    for spender in spenders {
        let allowance_balance_res = query_allowance_balance(&contract, &owner_address.to_owned(), &spender).await;

        // check back results
        let allowance_balance = match allowance_balance_res {
            Ok(res) => res,
            Err(e) => {
                let err_msg = format!("Error querying for allowance balance for contract-addr={}, owner-addr={}, spender-addr={}; err={}", contract_address, owner_address, &spender, e);
                return Err( (contract_address, err_msg) );
            }
        };

        // floating-point ready representation for U256
        let allowance_bal_fp = match BSCU256::from_dec_str(&allowance_balance.to_string()) {
            Ok(res) => res,
            Err(e) => {
                let err_msg = format!("Error converting from web3::types::U256 to bscscan::prelude::U256 for floating-point representation ability; err={}", e);
                return Err( (contract_address, err_msg) );
            }
        };

        result_struct.spender_allowances.insert(spender.to_owned(), allowance_bal_fp.to_f64_lossy() / 10_f64.powf(result_struct.decimals.into()));
    }

    Ok(result_struct)
}

/// Select and return api key for selected chain type.
/// The program needs environment variables as follows to be defined to cover
/// all API platforms which one of them will be used at runtime depending on
/// which chain has been selected.
///
/// * `bsc` - require environment variable `TRACPLS_BSCSCAN_APIKEY`
/// * `ethereum` - require environment variable `TRACPLS_ETHERSCAN_APIKEY`
/// * `polygon` - require environment variable `TRACPLS_POLYGONSCAN_APIKEY`
///
/// If such environment variable after selected has not defined yet, then
/// this function will panic.
///
/// # Arguments
/// * `chain` - chain type
fn select_apikey(chain: ChainType) -> String {
    match chain {
        ChainType::BSC => std::env::var("APPVKEK_BSCSCAN_APIKEY").expect("Required environment variable 'APPVKEK_BSCSCAN_APIKEY' to be defined"),
        ChainType::Ethereum => std::env::var("APPVKEK_ETHERSCAN_APIKEY").expect("Required environment variable 'APPVKEK_ETHERSCAN_APIKEY' to be defined"),
        ChainType::Polygon => std::env::var("APPVKEK_POLYGONSCAN_APIKEY").expect("Required environment variable 'APPVKEK_POLYGONSCAN_APIKEY' to be defined"),
    }
}

#[tokio::main]
async fn main() {
    let cmd_args = CommandlineArgs::parse();

    // validate value of chain flag option
    let chain_value = cmd_args.chain.to_lowercase();
    let chain: Option<ChainType>;
    if chain_value == "bsc" {
        chain = Some(ChainType::BSC);
    }
    else if chain_value == "ethereum" {
        chain = Some(ChainType::Ethereum);
    }
    else if chain_value == "polygon" {
        chain = Some(ChainType::Polygon);
    }
    else {
        eprintln!("Error invalid value for --chain.
Possible values are 'bsc', 'ethereum', or 'polygon'.");
        std::process::exit(1);
    }

    let web3 = create_web3(chain.unwrap()); 
    // check if input address is in correct format, and is actually EOA
    match perform_check_is_eoa(&web3, &cmd_args.address).await {
        Ok(is_eoa) => {
            if !is_eoa {
                eprintln!("Error input address is not EOA");
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    let ctx = Context::create(chain.unwrap(), select_apikey(chain.unwrap()));
    let accounts = evmscan::accounts();

    // HashMap for token contract to HashMap of spender addresses
    type DummyType = u8;
    const DUMMY_VALUE: DummyType = 0;
    let mut ct_txs: HashMap<String, HashMap<String, DummyType>> = HashMap::new();
    // make sure to make it lowercased.
    let owner_address = cmd_args.address.to_lowercase();

    #[allow(dead_code)]
    let mut start_time = std::time::Instant::now();
    if cmd_args.execution_time {
        measure_start(&mut start_time);
    }

    // get all transactions
    match accounts.get_list_normal_transactions(&ctx, &owner_address) {
        Ok(txs) => {
            for tx in txs {
                // 0x095ea7b3 is method-id for approve method
                if &tx.from == &owner_address && !tx.is_error && tx.input.starts_with("0x095ea7b3") {
                    if !ct_txs.contains_key(&tx.to) {
                        ct_txs.insert(tx.to.to_owned(), HashMap::new());
                    }

                    // get the spender from the first argument
                    let arguments = match parse_256_method_arguments(&tx.input) {
                        Ok(res) => {
                            // it should contains at least 2 elements
                            // method-id, spender, and amount for approve() method
                            if res.len() < 2 {
                                eprintln!("Error parsing arguments for hex-string from approve() method call.
It should contain at least three arguments for approve() method signature.");
                                std::process::exit(1);
                            }

                            res
                        },
                        Err(e) => {
                            eprintln!("Error parsing arguments of {}; err={}", tx.to, e);
                            std::process::exit(1);
                        }
                    };

                    // cleanup first argument to get address (64 chars to 40 chars
                    // by remove first 24 chars)
                    let mut spender_addr = arguments[0][24..].to_owned();
                    spender_addr.insert_str(0, "0x");

                    if let Some(val_hashmap) = ct_txs.get_mut(&tx.to) {
                        // use index-0 as it is spender address
                        if !(*val_hashmap).contains_key(&spender_addr) {
                            (*val_hashmap).insert(spender_addr, DUMMY_VALUE);
                        }
                    }
                }
            }
        },
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    // to avoid rate limit, this number would change if use different public node
    // experimentation, or consulting document for rate limit is needed
    const RPC_RATE_LIMIT: usize = 2000;
    let num_outputs_array = (ct_txs.len() as f64 / RPC_RATE_LIMIT as f64).ceil() as usize;
    let mut running_added_item = 0;
    
    // convert HashMap into Vec of tuple
    let ct_txs_vec = Vec::from_iter(ct_txs.into_iter().map(|(key,val)| (key,val)));

    for i in 0..num_outputs_array {
        let mut outputs = Vec::with_capacity(RPC_RATE_LIMIT);
    
        // collect items for each chunk
        while running_added_item < RPC_RATE_LIMIT && i * RPC_RATE_LIMIT + running_added_item < ct_txs_vec.len() {
            let (ct, spenders) = &ct_txs_vec[i * RPC_RATE_LIMIT + running_added_item];

            let spenders_collected = spenders.clone().into_keys().collect::<Vec::<String>>();

            outputs.push(query(&web3, ct.to_owned(), owner_address.to_owned(), spenders_collected));
            running_added_item = running_added_item + 1;
        }

        // async and wait
        let results = futures::future::join_all(outputs).await;
        for res in results {
            match res {
                Ok(r) => {
                    println!("[{}] {}", r.name, r.address);
                    for (spender, allowance) in r.spender_allowances {
                        println!("  * {} - {}", spender, allowance);
                    }
                },
                Err(e) => {
                    println!("[Error] {} - {}", e.0, e.1);
                }
            }
        }

        if i * RPC_RATE_LIMIT + running_added_item >= ct_txs_vec.len() {
            break;
        }

        // reset states
        running_added_item = 0;
    }

    if cmd_args.execution_time {
        measure_end(&start_time, true);
    }
}
