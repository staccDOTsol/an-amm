/// This submodule is dedicated to managing the acceptance of a new admin in the Automated Market Maker (AMM).
/// It includes functions and structures necessary for processing admin acceptance transactions, ensuring that
/// the new admin is correctly recorded and integrated into the AMM's administrative controls.
pub mod accept_admin {

    // BuyEvent event
    #[event]
    pub struct AcceptAdminEvent {
        pub proposed_admin: Pubkey,
        pub timestamp: u64,
    }

    use crate::AcceptAdmin;
    use anchor_lang::prelude::*;

    /// Accepts a proposed change to admin
    ///
    /// # Parameters:
    /// - `ctx`: Context containing all required accounts for the transaction.
    /// # Returns:
    /// - Result indicating success or an error.

    pub fn handler(ctx: Context<AcceptAdmin>) -> Result<()> {
        let global_parameters = &mut ctx.accounts.global_parameters;
        global_parameters.admin = ctx.accounts.admin.key();
        global_parameters.proposed_admin = Pubkey::default();
        // Emit the event
        emit_cpi!(AcceptAdminEvent {
            proposed_admin: *ctx.accounts.admin.to_account_info().key,
            timestamp: Clock::get()?.unix_timestamp as u64,
        });
        emit!(AcceptAdminEvent {
            proposed_admin: *ctx.accounts.admin.to_account_info().key,
            timestamp: Clock::get()?.unix_timestamp as u64,
        });

        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::fixtures::tests::setup_test_environment;
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use switchboard_solana::Instruction;
    use switchboard_solana::Pubkey;
    async fn accept_admin_instruction(
        proposed_admin_pubkey: Pubkey,
        global_parameters_pubkey: Pubkey,
    ) -> Instruction {
        let data = switchboard_solana::get_ixn_discriminator("accept_admin").to_vec();
        // Construct the accounts required for the add_liquidity_instruction
        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(global_parameters_pubkey, false),
            solana_sdk::instruction::AccountMeta::new(proposed_admin_pubkey, true),
            solana_sdk::instruction::AccountMeta::new_readonly(
                Pubkey::from_str("38C9cb9ak6zRdtA3ZxKPp9sYAPEKT9KfZcUcdC5Tda69").unwrap(),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new_readonly(crate::ID, false),
        ];
        // Create the instruction using the program_id, accounts, and data
        solana_sdk::instruction::Instruction {
            program_id: crate::ID,
            accounts,
            data,
        }
    }

    async fn propose_admin_instruction(
        proposed_admin_pubkey: Pubkey,
        admin_pubkey: Pubkey,
        global_parameters_pubkey: Pubkey,
    ) -> Instruction {
        let data = switchboard_solana::get_ixn_discriminator("propose_admin").to_vec();
        // Construct the accounts required for the add_liquidity_instruction
        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(global_parameters_pubkey, false),
            solana_sdk::instruction::AccountMeta::new(admin_pubkey, true),
            solana_sdk::instruction::AccountMeta::new_readonly(proposed_admin_pubkey, false),
            solana_sdk::instruction::AccountMeta::new_readonly(
                Pubkey::from_str("38C9cb9ak6zRdtA3ZxKPp9sYAPEKT9KfZcUcdC5Tda69").unwrap(),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new_readonly(crate::ID, false),
        ];
        // Create the instruction using the program_id, accounts, and data
        solana_sdk::instruction::Instruction {
            program_id: crate::ID,
            accounts,
            data,
        }
    }

    #[tokio::test]
    async fn accept_admin_instruction_test() {
        let setup = setup_test_environment(true).await;
        let global_parameters_pubkey = setup.global_parameters;
        let proposed_admin = Keypair::new();
        let admin_pubkey = setup.keypair.pubkey();
        let proposed_admin_instruction = propose_admin_instruction(
            proposed_admin.pubkey(),
            admin_pubkey,
            global_parameters_pubkey,
        )
        .await;
        let accept_admin_instruction =
            accept_admin_instruction(proposed_admin.pubkey(), global_parameters_pubkey).await;

        let mut tx = Transaction::new_with_payer(
            &[proposed_admin_instruction, accept_admin_instruction],
            Some(&setup.keypair.pubkey()),
        );
        tx.sign(
            &[&setup.keypair, &proposed_admin],
            setup.client.get_latest_blockhash().await.unwrap(),
        );
        let result = setup.client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", result);
        assert!(result.is_ok(), "Accept admin transaction should succeed");
    }
}
