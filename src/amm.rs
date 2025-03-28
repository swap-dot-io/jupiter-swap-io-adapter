
use anyhow::Result;
use jupiter_amm_interface::{
    AccountMap, Amm, AmmContext, KeyedAccount, Quote, QuoteParams, Swap, SwapAndAccountMetas, SwapParams
};
use solana_sdk::{account::Account, pubkey::Pubkey};
// use swap_io_clmm::states::{AmmConfig, TickArrayBitmapExtension, TickArrayState};
use swap_io_clmm_rust_sdk::{instruction::InstructionBuilder, pool::{PoolManager, NEIGHBORHOOD_SIZE}, quote::QuoteCalculator};



#[derive(Clone)]
pub struct SwapIoClmmAdapter {
    pool_manager: PoolManager,
}

impl SwapIoClmmAdapter {
    fn new(pool_key: Pubkey, pool_state_account: &Account, program_id: Pubkey, epoch: u64) -> Result<Self> {
        let pool_manager = PoolManager::new(epoch, pool_key, program_id, pool_state_account)?;

        Ok(
        Self {
            pool_manager,
        })
    }

    pub fn get_up_tick_array_keys(&self) -> &Vec<Pubkey> {
        &self.pool_manager.up_tick_array_keys
    }
    pub fn get_down_tick_array_keys(&self) -> &Vec<Pubkey> {
        &self.pool_manager.down_tick_array_keys
    }

    pub fn pool_manager(&self) -> &PoolManager {
        &self.pool_manager
    }

    fn get_tick_arrays_accounts(&self, tick_array_keys: &Vec<Pubkey>, account_map: &AccountMap) -> Result<Vec<Account>> {
        let mut tick_arrays = vec![];
        for key in tick_array_keys.iter() {
            let tick_array_account = account_map
                .get(key)
                .ok_or_else(|| anyhow::anyhow!("TickArray account not found"))?;
            tick_arrays.push(tick_array_account.clone());
        }
        Ok(tick_arrays)
    }
}

impl Amm for SwapIoClmmAdapter
where
    Self: Sized,
{
    fn from_keyed_account(keyed_account: &KeyedAccount, amm_context: &AmmContext) -> Result<Self> {
        let pool_key = keyed_account.key;
        let pool_data: &[u8] = keyed_account.account.data.as_ref();

        // Check if we have the 8-byte discriminator at the beginning
        if pool_data.len() < 8 {
            return Err(anyhow::anyhow!("Account data too short"));
        }

        let program_id = keyed_account.account.owner;
        Self::new(
            pool_key,
            &keyed_account.account,
            program_id,
            amm_context
                .clock_ref
                .epoch
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    fn label(&self) -> String {
        "SWAP_IO_CLMM".to_string()
    }

    fn program_id(&self) -> Pubkey {
        self.pool_manager.program_id
    }

    fn key(&self) -> Pubkey {
        self.pool_manager.pool_key
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        self.pool_manager.get_reserve_mints()
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        let mut result = vec![];
        let state = self.pool_manager.pool_state;
        result.push(state.amm_config);
        result.push(state.token_mint_0);
        result.push(state.token_mint_1);
        // TickArrayBitmapExtension
        result.push(self.pool_manager.tick_array_bitmap_extension());
        // TickArrays
        result.extend_from_slice(&self.pool_manager.up_tick_array_keys);
        result.extend_from_slice(&self.pool_manager.down_tick_array_keys);
        result
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        let amm_config_account = account_map
            .get(&self.pool_manager.pool_state.amm_config)
            .ok_or_else(|| anyhow::anyhow!("AmmConfig account not found"))?;
        // Store the token data in the struct
        let mint0_account = account_map
            .get(&self.pool_manager.pool_state.token_mint_0)
            .ok_or_else(|| anyhow::anyhow!("Mint0 account not found"))?;
        let mint1_data = account_map
            .get(&self.pool_manager.pool_state.token_mint_1)
            .ok_or_else(|| anyhow::anyhow!("Mint1 account not found"))?;
        let tickarray_bitmap_extension_account = account_map
            .get(&self.pool_manager.tick_array_bitmap_extension())
            .ok_or_else(|| anyhow::anyhow!("TickArrayBitmapExtension account not found"))?;
        
        let up_ticks_accounts = self.get_tick_arrays_accounts(&self.pool_manager.up_tick_array_keys, account_map)?;
        let down_ticks_accounts = self.get_tick_arrays_accounts(&self.pool_manager.down_tick_array_keys, account_map)?;
        self.pool_manager.update(vec![amm_config_account, mint0_account, mint1_data, tickarray_bitmap_extension_account], up_ticks_accounts, down_ticks_accounts)
    }

    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
        let quote = QuoteCalculator::calculate_quote(
            quote_params.input_mint,
            quote_params.output_mint,
            quote_params.swap_mode == jupiter_amm_interface::SwapMode::ExactIn,
            quote_params.amount,
            &self.pool_manager)?;
        Ok(Quote {
            fee_pct: quote.fee_pct,
            in_amount: quote.in_amount,
            out_amount: quote.out_amount,
            fee_amount: quote.fee_amount,
            fee_mint: quote.fee_mint,
            ..Quote::default()
        })

    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let instruction = InstructionBuilder::build_swap_instruction(&self.pool_manager, swap_params.source_mint, swap_params.destination_mint, swap_params.source_token_account, swap_params.destination_token_account)?;
        let account_metas = instruction.accounts;
        Ok(SwapAndAccountMetas {
            swap: Swap::RaydiumClmmV2,
            account_metas,
        })
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }

    fn has_dynamic_accounts(&self) -> bool {
        false
    }

    fn requires_update_for_reserve_mints(&self) -> bool {
        false
    }

    fn supports_exact_out(&self) -> bool {
        true
    }

    fn get_user_setup(&self) -> Option<jupiter_amm_interface::AmmUserSetup> {
        None
    }

    fn unidirectional(&self) -> bool {
        false
    }

    fn program_dependencies(&self) -> Vec<(Pubkey, String)> {
        std::vec![]
    }

    fn get_accounts_len(&self) -> usize {
        let base_acounts = 13; //with signer
        let tick_arrsy_bitmap_extension = 1;
        let tick_array_accounts = NEIGHBORHOOD_SIZE;
        base_acounts + tick_arrsy_bitmap_extension + tick_array_accounts as usize
    }

    fn underlying_liquidities(&self) -> Option<std::collections::HashSet<Pubkey>> {
        None
    }

    fn is_active(&self) -> bool {
        true
    }
}
