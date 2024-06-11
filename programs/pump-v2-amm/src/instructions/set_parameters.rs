use crate::error::ErrorCode;
use anchor_lang::prelude::*;

/// Adjusts the AMM's operational parameters including protocol fees, referrer fees, and discounts.
/// This function ensures that the referrer discount does not exceed the referrer fee itself.
/// It updates the global parameters of the AMM with the new fee settings provided.
pub mod set_parameters {
    use crate::SetParameters;

    use super::*;

    /// Sets the parameters of the AMM
    ///
    /// # Parameters:
    /// - `ctx`: The context of the AMM.
    /// - `protocol_fee_bps`: The fee rate for the protocol in basis points.
    /// - `referrer_fee_bps`: The fee rate for the referrer in basis points.
    /// - `referrer_fee_discount_bps`: The discount rate for the referrer fee in basis points.
    ///
    /// # Returns:
    /// - `Result<()>`: A success indicator if the parameters are set successfully.
    pub fn handler(
        ctx: Context<SetParameters>,
        protocol_fee_bps: u64,
        referrer_fee_bps: u64,
        referrer_fee_discount_bps: u64,
    ) -> Result<()> {
        require!(
            referrer_fee_discount_bps > referrer_fee_bps,
            ErrorCode::ReferrerFeeDiscountExceedsFee
        );
        require!(
            protocol_fee_bps > referrer_fee_discount_bps + referrer_fee_bps,
            ErrorCode::InvalidFeeConfiguration
        );

        let amm_params = &mut ctx.accounts.global_parameters;
        amm_params.protocol_fee_bps = protocol_fee_bps;
        amm_params.referrer_fee_bps = referrer_fee_bps;
        amm_params.referrer_fee_discount_bps = referrer_fee_discount_bps;

        Ok(())
    }
}
