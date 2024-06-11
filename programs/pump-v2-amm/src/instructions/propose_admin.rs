/// This submodule is dedicated to managing the proposal of a new admin in the Automated Market Maker (AMM).
/// It includes functions and structures necessary for processing admin proposal transactions, ensuring that
/// the proposed admin is correctly recorded and prepared for potential integration into the AMM's administrative controls.
pub mod propose_admin {
    use crate::ProposeAdmin;
    use anchor_lang::prelude::*;

    // BuyEvent event
    #[event]
    pub struct ProposeAdminEvent {
        pub proposed_admin: Pubkey,
        pub timestamp: u64,
    }

    /// Proposes a new admin
    ///
    /// # Parameters:
    /// - `ctx`: Context containing all required accounts for the transaction.
    /// # Returns:
    /// - Result indicating success or an error.

    pub fn handler(ctx: Context<ProposeAdmin>) -> Result<()> {
        let global_parameters = &mut ctx.accounts.global_parameters;
        global_parameters.proposed_admin = ctx.accounts.proposed_admin.key();

        // Emit the event
        emit_cpi!(ProposeAdminEvent {
            proposed_admin: *ctx.accounts.proposed_admin.to_account_info().key,
            timestamp: Clock::get()?.unix_timestamp as u64,
        });

        emit!(ProposeAdminEvent {
            proposed_admin: *ctx.accounts.proposed_admin.to_account_info().key,
            timestamp: Clock::get()?.unix_timestamp as u64,
        });

        Ok(())
    }
}
