use clap::Parser;
use std::collections::HashMap;

#[derive(Debug, Parser)]
#[clap(author="Wasin Thonkaew (wasin@wasin.io)")]
#[clap(name="appvkek")]
#[clap(about="cli tool to check your approval and allowance associated with token contract addresses out there")]
pub struct CommandlineArgs {
    /// User's wallet address to check against.
    #[clap(long="wallet-address", short='a', required=true)]
    pub address: String,
}

/// Top-level meta information.
#[derive(Debug)]
pub struct TokenContractWithSpenderAllowances {
    /// Contract name
    pub name: String,

    /// Contract address
    pub address: String,

    /// Number of decimals to token
    pub decimals: u8,

    /// Hash map of spender with its associated allowance balance
    /// It would be possible to hold maximum allowance value as maximum value of
    /// `f64` is `1.7976931348623157e+308_f64`.
    pub spender_allowances: HashMap<String, f64>,
}
