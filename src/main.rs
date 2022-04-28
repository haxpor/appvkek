use ::bscscan::bscscan;
use ::bscscan::environ::Context;
use ::bscscan::prelude::U256 as BSCU256;    // for floating-point representation for U256
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

    if name.is_err() || decimals.is_err() {
        return Err( (contract_address, "Error in querying top-level query".to_owned()) );
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

#[tokio::main]
async fn main() {
    let cmd_args = CommandlineArgs::parse();
    let web3 = create_web3(); 

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

    let ctx = Context { api_key: std::env::var("APPVKEK_BSCSCAN_APIKEY").expect("Required environment variable 'APPVKEK_BSCSCAN_APIKEY' to be defined") };
    let accounts = bscscan::accounts();

    // HashMap for token contract to HashMap of spender addresses
    type DummyType = u8;
    const DUMMY_VALUE: DummyType = 0;
    let mut ct_txs: HashMap<String, HashMap<String, DummyType>> = HashMap::new();
    let owner_address = cmd_args.address;

    #[allow(dead_code)]
    let mut start_time = std::time::Instant::now();
    if !cmd_args.no_execution_time {
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

    let mut asyncs = Vec::new();
    for (ct, spenders) in ct_txs {
        let spenders_collected: Vec<String> = spenders.into_keys().collect();
        asyncs.push(query(&web3, ct.to_owned(), owner_address.to_owned(), spenders_collected));
    }

    let results = futures::future::join_all(asyncs).await;
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
    if !cmd_args.no_execution_time {
        measure_end(&start_time, true);
    }
}
