use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid operation.")]
    InvalidOperation,
    #[msg("Insufficient liquidity for this operation.")]
    InsufficientLiquidity,
    #[msg("Invalid token amounts for liquidity addition.")]
    InvalidTokenAmounts,
    #[msg("Liquidity removal exceeds available shares.")]
    LiquidityRemovalExceedsShares,
    #[msg("Requested buy amount results in zero output.")]
    ZeroOutputForBuy,
    #[msg("Requested sell amount results in zero output.")]
    ZeroOutputForSell,
    #[msg("Parameters setting failed.")]
    ParametersSettingFailed,
    #[msg("Insufficient liquidity minted.")]
    InsufficientLiquidityMinted,
    #[msg("Reserve balance mismatch.")]
    ReserveBalanceMismatch,
    #[msg("Insufficient quote amount requested.")]
    InsufficientQuoteAmount,
    #[msg("Already initialized.")]
    AlreadyInitialized,
    #[msg("Math overflow.")]
    MathOverflow,
    #[msg("Token transfer failed.")]
    TokenTransferFailed,
    #[msg("Invalid referrer account.")]
    InvalidReferrerAccount,
    #[msg("Invalid referrer mint.")]
    InvalidReferrerMint,
    #[msg("Quote amount too low.")]
    QuoteAmountTooLow,
    #[msg("Base amount too low.")]
    BaseAmountTooLow,
    #[msg("Referrer fee discount exceeds fee.")]
    ReferrerFeeDiscountExceedsFee,
    #[msg("Invalid fee configuration.")]
    InvalidFeeConfiguration,
}
