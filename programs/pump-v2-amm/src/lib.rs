use anchor_lang::prelude::*;
pub mod error;
pub mod fixtures;
pub mod instructions;
pub mod state;
pub mod utils;
use state::*;

declare_id!("GuA9pdtCqntcxCCWr2W8WbD7YdN7RDCni1JrFbzibENg");

#[program]
pub mod solana_amm {
    use super::*;

    pub fn create(ctx: Context<Create>) -> Result<()> {
        instructions::create::create::handler(ctx)
    }

    pub fn propose_admin(ctx: Context<ProposeAdmin>) -> Result<()> {
        instructions::propose_admin::propose_admin::handler(ctx)
    }

    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        instructions::accept_admin::accept_admin::handler(ctx)
    }

    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        base_amount: u64,
        quote_amount: u64,
        min_lp_shares: u64,
    ) -> Result<()> {
        instructions::add_liquidity::add_liquidity::handler(
            ctx,
            base_amount,
            quote_amount,
            min_lp_shares,
        )
    }

    pub fn remove_liquidity(
        ctx: Context<RemoveLiquidity>,
        shares: u64,
        quote_min_amount: u64,
        base_min_amount: u64,
    ) -> Result<()> {
        instructions::remove_liquidity::remove_liquidity::handler(
            ctx,
            shares,
            quote_min_amount,
            base_min_amount,
        )
    }
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::initialize::handler(ctx)
    }
    pub fn buy<'a>(
        ctx: Context<'_, '_, '_, 'a, Buy<'a>>,
        base_amount: u64,
        max_quote_amount: u64,
    ) -> Result<()> {
        instructions::buy::buy::handler(ctx, base_amount, max_quote_amount)
    }

    pub fn sell<'a>(
        ctx: Context<'_, '_, '_, 'a, Sell<'a>>,
        base_amount: u64,
        max_quote_amount: u64,
    ) -> Result<()> {
        instructions::sell::sell::handler(ctx, base_amount, max_quote_amount)
    }

    pub fn set_parameters(
        ctx: Context<SetParameters>,
        protocol_fee_bps: u64,
        referrer_fee_bps: u64,
        referrer_rebate_bps: u64,
    ) -> Result<()> {
        instructions::set_parameters::set_parameters::handler(
            ctx,
            protocol_fee_bps,
            referrer_fee_bps,
            referrer_rebate_bps,
        )
    }
}
