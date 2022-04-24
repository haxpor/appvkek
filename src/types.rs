use clap::Parser;

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
pub struct TopLevelContractMetaInfo {
    /// Contract name
    pub name: String,

    /// Number of decimals to token
    pub decimals: u8,
}

/// Inferred information through HashMap towards approved contract address.
/// It is inferred because we can get major information of which address
/// through key(s) in HashMap which store this structure.
pub struct InferredContractMetaInfo {
    /// Allowance balance for user's address towards this inferred contract, spender
    /// and owner address.
    pub allowance_balance: f64,
}
