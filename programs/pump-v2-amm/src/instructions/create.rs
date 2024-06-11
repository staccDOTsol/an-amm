use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
// CreateEvent event
#[event]
pub struct CreateEvent {
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub timestamp: u64,
    pub user: Pubkey,
}
// This section defines the `create` module, which is responsible for encapsulating the logic necessary to initialize and configure a new Automated Market Maker (AMM) instance. This includes setting up the foundational parameters such as mint addresses for the base, quote, and liquidity provider tokens, and ensuring that the AMM is ready for subsequent operations like adding liquidity, trading, etc.
pub mod create {
    use super::*;
    use crate::Create; // Import the Create instruction structure // Import other necessary components from the parent module

    /// Handles the initialization of an AMM, setting up its basic components such as mint addresses.
    ///
    /// # Parameters:
    /// - `ctx`: Context containing all the accounts required for AMM initialization.
    ///
    /// # Returns:
    /// - Result indicating success or an error.
    pub fn handler(ctx: Context<Create>) -> Result<()> {
        // Initialize AMM state and populate fields
        let amm = &mut ctx.accounts.amm;
        amm.base_mint = ctx.accounts.base_mint.key();
        amm.quote_mint = ctx.accounts.quote_mint.key();
        amm.lp_mint = ctx.accounts.lp_mint.key();
        amm.creator = ctx.accounts.user.key();
        amm.base_reserve_ata = ctx.accounts.base_reserve_ata.key();
        amm.quote_reserve_ata = ctx.accounts.quote_reserve_ata.key();
        amm.fee_receiver_ata = ctx.accounts.fee_receiver_ata.key();
        invoke(
            &spl_associated_token_account::instruction::create_associated_token_account(
                &ctx.accounts.user.to_account_info().key,
                &ctx.accounts.amm.to_account_info().key,
                &ctx.accounts.base_mint.to_account_info().key,
                &ctx.accounts.base_token_program.to_account_info().key,
            ),
            &[
                ctx.accounts.associated_token_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.base_reserve_ata.to_account_info(),
                ctx.accounts.user.to_account_info(),
                ctx.accounts.amm.to_account_info(),
                ctx.accounts.base_mint.to_account_info(),
                ctx.accounts.base_token_program.to_account_info(),
            ],
        )?;
        invoke(
            &spl_associated_token_account::instruction::create_associated_token_account(
                &ctx.accounts.user.to_account_info().key,
                &ctx.accounts.amm.to_account_info().key,
                &ctx.accounts.quote_mint.to_account_info().key,
                &ctx.accounts.quote_token_program.to_account_info().key,
            ),
            &[
                ctx.accounts.associated_token_program.to_account_info(),
                ctx.accounts.rent.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.quote_reserve_ata.to_account_info(),
                ctx.accounts.user.to_account_info(),
                ctx.accounts.amm.to_account_info(),
                ctx.accounts.quote_mint.to_account_info(),
                ctx.accounts.quote_token_program.to_account_info(),
            ],
        )?;
        // Emit the event
        emit_cpi!(CreateEvent {
            base_mint: *ctx.accounts.base_mint.to_account_info().key,
            quote_mint: *ctx.accounts.quote_mint.to_account_info().key,
            lp_mint: *ctx.accounts.lp_mint.to_account_info().key,
            timestamp: Clock::get()?.unix_timestamp as u64,
            user: *ctx.accounts.user.to_account_info().key,
        });

        emit!(CreateEvent {
            base_mint: *ctx.accounts.base_mint.to_account_info().key,
            quote_mint: *ctx.accounts.quote_mint.to_account_info().key,
            lp_mint: *ctx.accounts.lp_mint.to_account_info().key,
            timestamp: Clock::get()?.unix_timestamp as u64,
            user: *ctx.accounts.user.to_account_info().key,
        });

        Ok(())
    }
}
