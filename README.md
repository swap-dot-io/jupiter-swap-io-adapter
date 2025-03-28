# Swap.io CLMM Jupiter Adapter

An adapter for integrating the Swap.io Concentrated Liquidity Market Maker (CLMM) protocol with the [Jupiter](https://jup.ag/) liquidity aggregator.

## Overview

This repository contains an implementation of the Jupiter AMM interface for the Swap.io CLMM protocol, allowing Jupiter users to access liquidity in Swap.io pools and execute swaps through the aggregator's unified interface.

## Features

- Full integration with Jupiter DEX aggregator
- Support for precise quotation calculations for CLMM pools
- Ability to execute swaps with both exact input and exact output
- Working with tick arrays for efficient price and liquidity determination

## Dependencies

- `jupiter-amm-interface`: Interface for Jupiter integration
- `swap_io_clmm_rust_sdk`: SDK for interacting with the Swap.io CLMM protocol
- `solana-sdk`: Core tools for working with Solana

## Usage

The adapter implements the `Amm` interface from `jupiter-amm-interface` and can be used according to Jupiter documentation. Main functions:

- `from_keyed_account`: Creating an adapter from a pool account
- `quote`: Getting a quote for a swap
- `get_swap_and_account_metas`: Generating instructions for executing a swap

## Example

```rust
use jupiter_amm_interface::{AccountMap, Amm, AmmContext, KeyedAccount, QuoteParams, SwapParams, SwapMode};
use solana_sdk::{pubkey::Pubkey, account::Account};

// 1. CREATE: Instantiate the adapter from a pool account
let pool_key = Pubkey::new_from_array([/* pool pubkey */]);
let keyed_account = KeyedAccount {
    key: pool_key,
    account: Account::new(/* pool account data */),
};
let amm_context = AmmContext {
    /* context information */
};
let mut adapter = SwapIoClmmAdapter::from_keyed_account(&keyed_account, &amm_context)?;

// 2. GET ACCOUNTS: Retrieve the accounts that need to be updated
let accounts_to_update = adapter.get_accounts_to_update();
// Fetch these accounts from the blockchain (not shown)
let mut account_map = AccountMap::new();
// Populate account_map with fetched accounts
// account_map.insert(pubkey, keyed_account);

// 3. UPDATE: Update the adapter with latest on-chain data
adapter.update(&account_map)?;

// 4a. QUOTE: Get a price quote for a swap
let quote_params = QuoteParams {
    input_mint: token_a_mint,
    output_mint: token_b_mint,
    amount: 1_000_000,
    swap_mode: SwapMode::ExactIn,
};
let quote = adapter.quote(&quote_params)?;
println!("Expected output amount: {}", quote.out_amount);

// 4b. SWAP: Generate instructions for executing the swap
let swap_params = SwapParams {
    source_mint: token_a_mint,
    destination_mint: token_b_mint,
    source_token_account: user_token_a_account,
    destination_token_account: user_token_b_account,
};
let swap_instruction = adapter.get_swap_and_account_metas(&swap_params)?;
// Now you can include this instruction in a Solana transaction
```

## Workflow

The typical workflow for using this adapter follows these steps:

1. **Create** - Instantiate the adapter from a pool account using `from_keyed_account`
2. **Get Accounts** - Get relevant accounts using `get_accounts_to_update` (returns necessary account public keys)
3. **Update** - Update the adapter with latest on-chain data using `update` method
4. **Quote or Swap** - Either get a price quote with `quote` or generate swap instructions with `get_swap_and_account_metas`

This workflow ensures that the adapter always operates with the most up-to-date pool state and price information.

## License

[MIT](LICENSE)

