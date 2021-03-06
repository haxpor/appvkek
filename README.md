# appvkek
Cli tool to check your approved permission with contracts connected with your wallet address, and allow to disapprove them. Support BSC (first), then Ethereum, and Polygon chain.

# Brief
The goal of this tool is to list out potential token contracts which are approved
with allowance such that user can externally set a new amount of allowance later.

At first, I planned to include an automated feature to disapprove all non-zero
allowance balance. But that would be too much destructive, and risky for gas
fees to blowing up unexpectedly.

So for now, I've planned to implement another tool to help as an executor of
smart contract's method against the target contract address. So it is safer and
separated in sense of responsibility. I'll update more later when such project
comes into fruition.

# Setup

Grab bscscan.com API key then define it via environment variable namely `APPVKEK_BSCSCAN_APIKEY` before running the application.

# Usage

Use the following command.

```bash
$ appvkek -a 0xcab1067285d391d58891065de2f83776603b2667
[NS] 0x62accaecc139ba155c78f6134f174e7b0c8761c4
  * 0x10ed43c718714eb63d5aa57b78b54704e256024e - 115792089237316200000000000000000000000000000000000000000000000000000000000
[MGAIN] 0xc6ab6599fd5dbbbf106a316f8f732d65e4ecd134
  * 0x10ed43c718714eb63d5aa57b78b54704e256024e - 115792089237316200000000000000000000000000000000000000000000000000000000000
[PNFT] 0x6d66db8f70bbedcadc5b91241cd55b966177ebea
  * 0x10ed43c718714eb63d5aa57b78b54704e256024e - 115792089237316200000000000000000000000000000000000000000000000000000000000
[MINI] 0xf85f8c31991c08c9085f83d2cc1b0818faf1064f
  * 0x10ed43c718714eb63d5aa57b78b54704e256024e - 115792089237316200000000000000000000000000000000000000000000000000000000000
[MLAND] 0x0e0d62e535a23aef8a82b20430faf55c68a06612
  * 0x10ed43c718714eb63d5aa57b78b54704e256024e - 115792089237316200000000000000000000000000000000000000000000000000000000000
[Meta IN] 0x9409eaa3cec6bf1b64c9b7b0097dc6cd7e30b731
  * 0x10ed43c718714eb63d5aa57b78b54704e256024e - 115792089237316200000000000000000000000000000000000000000000000000000000000
[MetaSWAP] 0xb27927d8f99527f1cdc46dd32e86efe1a9199e28
  * 0x10ed43c718714eb63d5aa57b78b54704e256024e - 115792089237316200000000000000000000000000000000000000000000000000000
[GS] 0x0900a50799c0a3d8132f1833cf002414d392613f
  * 0x10ed43c718714eb63d5aa57b78b54704e256024e - 115792089237316200000000000000000000000000000000000000000000000000000000000
[ELONMOON] 0xf642937ddddeb3c134bce69ca58175ff4b58dc1d
  * 0x10ed43c718714eb63d5aa57b78b54704e256024e - 115792089237316200000000000000000000000000000000000000000000000000000000000
...
[BUSD Token] 0xe9e7cea3dedca5984780bafc599bd69add087d56
  * 0x11111112542d85b3ef69ae05771c2dccff4faa26 - 115792089237316200000000000000000000000000000000000000000000
  * 0x10ed43c718714eb63d5aa57b78b54704e256024e - 115792089237316200000000000000000000000000000000000000000000
...
```

such wallet address is the top whale in BSC chain. We use it as an example only,
not to imply anything.

This will output the allowance balance associated with each spender address under
the token contract.

# Required Flags

* `--chain` (or `-c`) - possible values are `bsc`, `ethereum`, or `polygon` affecting the specified address.

# Optional Flags

* `--execution-time` - to also show the execution time for all processing, queries, etc.

# License
MIT, Wasin Thonkaew
