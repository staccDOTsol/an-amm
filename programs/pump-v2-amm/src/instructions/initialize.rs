use crate::error::ErrorCode;
use anchor_lang::prelude::*;

/// This section defines the `initialize` module, which is responsible for initializing the global parameters of the AMM.
/// This includes setting the admin key, which is the key responsible for managing the AMM,
/// and ensuring that the AMM is ready for operations like creating new AMMs, adding liquidity, etc.
pub mod initialize {
    use super::*;
    use crate::Initialize;

    pub fn handler(ctx: Context<Initialize>) -> Result<()> {
        let global_parameters = &mut ctx.accounts.global_parameters;
        require!(
            global_parameters.admin == Pubkey::default(),
            ErrorCode::AlreadyInitialized
        );

        global_parameters.admin = ctx.accounts.admin.key();

        Ok(())
    }
}
