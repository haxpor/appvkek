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

    // get all transactions
    match accounts.get_list_normal_transactions(&ctx, &cmd_args.address) {
        Ok(txs) => {
            let owner_addr = &cmd_args.address;

            let mut contract_toplevel_meta: HashMap<String, TopLevelContractMetaInfo> = HashMap::new();
            let mut contract_addrs: HashMap<String, HashMap<String, InferredContractMetaInfo>> = HashMap::new();

            for tx in txs {
                // 0x095ea7b3 is method-id for approve method
                if &tx.from == owner_addr &&
                   !tx.is_error &&
                   tx.input.starts_with("0x095ea7b3") {
                    // NOTE: approve() call can be for multiple of spenders but
                    // still should be the same contract address
                    if !contract_addrs.contains_key(&tx.to) {
                        contract_addrs.insert(tx.to.clone(), HashMap::new());
                    }

                    // create contract to interact with
                    // FIXME: creation of this might not be used at all
                    let contract = match create_contract(&web3, &tx.to, &ABI_STR) {
                        Ok(res) => res,
                        Err(e) => {
                            eprintln!("{}", e);
                            std::process::exit(1);
                        }
                    };

                    // save top-level meta info only if necessary so we don't
                    // duplicately make RPC request when data is already there
                    if !contract_toplevel_meta.contains_key(&tx.to) {
                        // query for its top-level information e.g. name, and decimals 
                        let name_fut = web3_query_no_params::<String>(&contract, "name");
                        let decimals_fut = web3_query_no_params::<u8>(&contract, "decimals");

                        let (name_res, decimals_res) = futures::join!(name_fut, decimals_fut);

                        let name = match name_res {
                            Ok(res) => res,
                            Err(e) => {
                                eprintln!("Error querying for name for contract-addr={}; err={}", &tx.to, e);
                                std::process::exit(1);
                            }
                        };
                        let decimals = match decimals_res {
                            Ok(res) => res,
                            Err(e) => {
                                eprintln!("Error querying for decimals for contract-addr={}; err={}", &tx.to, e);
                                std::process::exit(1);
                            }
                        };

                        contract_toplevel_meta.insert(tx.to.to_owned(), TopLevelContractMetaInfo {
                            name: name,
                            decimals: decimals
                        });
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

                    if let Some(val_hashmap) = contract_addrs.get_mut(&tx.to) {
                        // use index-0 as it is spender address
                        if !(*val_hashmap).contains_key(&spender_addr) {
                            // make query to get current allowanced balance
                            let allowance_balance_res = query_allowance_balance(&contract, owner_addr, &spender_addr).await;

                            // check back results
                            let allowance_balance = match allowance_balance_res {
                                Ok(res) => res,
                                Err(e) => {
                                    eprintln!("Error querying for allowance balance for contract-addr={}, owner-addr={}, spender-addr={}; err={}", &tx.to, owner_addr, &spender_addr, e);
                                    std::process::exit(1);
                                }
                            };

                            // floating-point ready representation for U256
                            let allowance_bal_fp = match BSCU256::from_dec_str(&allowance_balance.to_string()) {
                                Ok(res) => res,
                                Err(e) => {
                                    eprintln!("Error converting from web3::types::U256 to bscscan::prelude::U256 for floating-point representation ability; err={}", e);
                                    std::process::exit(1);
                                }
                            };

                            let inferred_meta = InferredContractMetaInfo {
                                allowance_balance: allowance_bal_fp.to_f64_lossy() / 10_f64.powf(contract_toplevel_meta[&tx.to].decimals.into())
                            };

                            (*val_hashmap).insert(spender_addr, inferred_meta);
                        }
                    }
                }
            }

            for (token_contract_addr, spenders_allowance) in contract_addrs {
                println!("[{}] {}", contract_toplevel_meta[&token_contract_addr].name, token_contract_addr);

                for (spender_addr, inferred_meta) in spenders_allowance {
                    println!("  * {} - {}", spender_addr, inferred_meta.allowance_balance);
                }
            }
        },
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
