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

    /// Whether to include execution time statistics at the end of procesing
    #[clap(long="execution-time", multiple_values=false, default_missing_value="true", takes_value=false)]
    pub execution_time: bool,

    /// Which chain to work with.
    /// Possible values are 'bsc', 'ethereum', and 'polygon'.
    #[clap(long="chain", short='c', required=true, multiple_values=false)]
    pub chain: String,
}

/// Top-level meta information.
#[derive(Debug, Clone)]
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
