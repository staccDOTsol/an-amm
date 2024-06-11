use crate::error::ErrorCode;
use crate::utils::{spl_token_transfer, TokenTransferParams};
use anchor_lang::prelude::*;

// RemoveLiquidityEvent event
#[event]
pub struct RemoveLiquidityEvent {
    pub base_amount: u64,
    pub quote_amount: u64,
    pub shares: u64,
    pub timestamp: u64,
    pub user: Pubkey,
}

/// Helper function to calculate the amount of base and quote tokens proportional to the shares being removed.
///
/// # Parameters:
/// - `shares`: Number of liquidity shares being removed.
/// - `amm`: Reference to the current state of the AMM.
///
/// # Returns:
/// - Result containing the tuple of base and quote tokens calculated or an error.
fn remove_quote(
    shares: u64,
    base_reserve: u64,
    quote_reserve: u64,
    total_shares: u64,
) -> Result<(u64, u64)> {
    let base_amount = (shares * base_reserve) / total_shares;
    let quote_amount = (shares * quote_reserve) / total_shares;
    Ok((base_amount, quote_amount))
}

/// The `remove_liquidity` module is responsible for handling the removal of liquidity from the AMM.
/// This includes calculating the amounts of base and quote tokens to be returned for the shares being removed,
/// ensuring the returned amounts meet the user's expectations, and updating the AMM's reserves accordingly.
pub mod remove_liquidity {
    use super::*;
    use crate::RemoveLiquidity;

    /// Handles the removal of liquidity by a user, ensuring minimum amounts are respected.
    ///
    /// # Parameters:
    /// - `ctx`: Context containing all necessary accounts for the transaction.
    /// - `shares`: The number of liquidity shares the user wants to remove.
    /// - `quote_min_amount`: The minimum amount of quote tokens the user expects to receive.
    /// - `base_min_amount`: The minimum amount of base tokens the user expects to receive.
    ///
    /// # Returns:
    /// - Result indicating success or an error.
    pub fn handler(
        ctx: Context<RemoveLiquidity>,
        shares: u64,
        quote_min_amount: u64,
        base_min_amount: u64,
    ) -> Result<()> {
        let base_mint = &ctx.accounts.base_mint;
        let quote_mint = &ctx.accounts.quote_mint;
        let user_lp_ata = &mut ctx.accounts.user_lp_ata;
        let base_reserve_ata = &mut ctx.accounts.base_reserve_ata;
        let quote_reserve_ata = &mut ctx.accounts.quote_reserve_ata;

        // Calculate the amount of base and quote tokens to be returned for the shares
        let (base_amount, quote_amount) = remove_quote(
            shares,
            ctx.accounts.amm.base_reserve,
            ctx.accounts.amm.quote_reserve,
            ctx.accounts.amm.total_shares,
        )?;

        // Ensure the returned amounts meet the user's expectations
        require!(
            quote_amount >= quote_min_amount,
            ErrorCode::QuoteAmountTooLow
        );
        require!(base_amount >= base_min_amount, ErrorCode::BaseAmountTooLow);

        // Update the AMM's reserves by subtracting the amounts to be removed
        let amm = &mut ctx.accounts.amm;
        amm.base_reserve -= base_amount;
        amm.quote_reserve -= quote_amount;
        amm.total_shares -= shares;

        // Define seeds for signing operations that require the AMM's authority
        let signer_seeds = [
            b"amm",
            ctx.accounts.amm.creator.as_ref(),
            base_mint.to_account_info().key.as_ref(),
            quote_mint.to_account_info().key.as_ref(),
            &[ctx.bumps.amm],
        ];

        // Transfer the base tokens from the AMM's reserves to the user's account
        let base_token_program = &ctx.accounts.base_token_program.to_account_info();
        let quote_token_program = &ctx.accounts.quote_token_program.to_account_info();

        // Burn the LP shares to reflect the removal of liquidity
        anchor_spl::token::burn(
            CpiContext::new(
                ctx.accounts.base_token_program.to_account_info(),
                anchor_spl::token::Burn {
                    mint: ctx.accounts.lp_mint.to_account_info(),
                    from: user_lp_ata.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            shares,
        )?;

        spl_token_transfer(TokenTransferParams {
            source: base_reserve_ata.to_account_info(),
            destination: ctx.accounts.user_base_ata.to_account_info(),
            amount: base_amount,
            authority: ctx.accounts.amm.to_account_info(),
            authority_signer_seeds: &signer_seeds,
            decimals: ctx.accounts.base_mint.decimals,
            mint: ctx.accounts.base_mint.to_account_info(),
            token_program: base_token_program.clone(),
        })?;

        // Transfer the quote tokens from the AMM's reserves to the user's account
        spl_token_transfer(TokenTransferParams {
            source: quote_reserve_ata.to_account_info(),
            destination: ctx.accounts.user_quote_ata.to_account_info(),
            amount: quote_amount,
            authority: ctx.accounts.amm.to_account_info(),
            authority_signer_seeds: &signer_seeds,
            decimals: ctx.accounts.quote_mint.decimals,
            mint: ctx.accounts.quote_mint.to_account_info(),
            token_program: quote_token_program.clone(),
        })?;

        // Emit the event
        emit_cpi!(RemoveLiquidityEvent {
            base_amount,
            quote_amount,
            shares,
            timestamp: Clock::get()?.unix_timestamp as u64,
            user: *ctx.accounts.user.to_account_info().key,
        });

        emit!(RemoveLiquidityEvent {
            base_amount,
            quote_amount,
            shares,
            timestamp: Clock::get()?.unix_timestamp as u64,
            user: *ctx.accounts.user.to_account_info().key,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::fixtures::tests::fetch_reserves;
    use crate::fixtures::tests::setup_amm;
    use crate::fixtures::tests::setup_mints_and_accounts;
    use crate::fixtures::tests::setup_test_environment;
    use crate::fixtures::tests::setup_user_accounts;
    use anchor_lang::AccountDeserialize;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use solana_transaction_status::option_serializer::OptionSerializer;
    use std::str::FromStr;
    use std::sync::Arc;
    use switchboard_solana::Pubkey;

    fn remove_liquidity_instruction(
        program_id: &solana_sdk::pubkey::Pubkey,
        amm_pubkey: &solana_sdk::pubkey::Pubkey,
        base_pubkey: &solana_sdk::pubkey::Pubkey,
        quote_pubkey: &solana_sdk::pubkey::Pubkey,
        payer_pubkey: &solana_sdk::pubkey::Pubkey,
        lp_mint_pubkey: &solana_sdk::pubkey::Pubkey,
        shares: u64,
        quote_min_amount: u64,
        base_min_amount: u64,
    ) -> solana_sdk::instruction::Instruction {
        let mut data = switchboard_solana::get_ixn_discriminator("remove_liquidity").to_vec();
        data.extend_from_slice(&shares.to_le_bytes());
        data.extend_from_slice(&quote_min_amount.to_le_bytes());
        data.extend_from_slice(&base_min_amount.to_le_bytes());
        // Construct the accounts required for the add_liquidity_instruction
        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(*amm_pubkey, false),
            solana_sdk::instruction::AccountMeta::new_readonly(*payer_pubkey, true),
            solana_sdk::instruction::AccountMeta::new(
                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &payer_pubkey,
                    &base_pubkey,
                    &spl_token_2022::ID,
                ),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new(
                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &payer_pubkey,
                    &quote_pubkey,
                    &spl_token::ID,
                ),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new(
                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &amm_pubkey,
                    &base_pubkey,
                    &spl_token_2022::ID,
                ),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new(
                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &amm_pubkey,
                    &quote_pubkey,
                    &spl_token::ID,
                ),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new(
                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &payer_pubkey,
                    &lp_mint_pubkey,
                    &spl_token::ID,
                ),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new(*lp_mint_pubkey, false),
            solana_sdk::instruction::AccountMeta::new(*base_pubkey, false),
            solana_sdk::instruction::AccountMeta::new(*quote_pubkey, false),
            solana_sdk::instruction::AccountMeta::new_readonly(spl_token_2022::ID, false),
            solana_sdk::instruction::AccountMeta::new_readonly(spl_token::ID, false),
            solana_sdk::instruction::AccountMeta::new_readonly(
                solana_program::system_program::ID,
                false,
            ),
            solana_sdk::instruction::AccountMeta::new_readonly(
                Pubkey::from_str("38C9cb9ak6zRdtA3ZxKPp9sYAPEKT9KfZcUcdC5Tda69").unwrap(),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new_readonly(*program_id, false),
        ];
        // Create the instruction using the program_id, accounts, and data
        solana_sdk::instruction::Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    }

    #[tokio::test]
    async fn test_remove_liquidity_success() {
        let setup = setup_test_environment(true).await;
        let shares = 10_000;

        let ix = remove_liquidity_instruction(
            &setup.program_id,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.lp_mint,
            shares,
            1, // minimum base tokens expected
            1, // minimum quote tokens expected
        );

        let mut tx = Transaction::new_with_payer(&[ix], Some(&setup.keypair.pubkey()));
        tx.sign(
            &[&setup.keypair],
            setup.client.get_latest_blockhash().await.unwrap(),
        );
        let result = setup.client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", result);
        assert!(
            result.is_ok(),
            "Remove liquidity transaction should succeed"
        );
    }
    use crate::instructions::remove_liquidity::remove_quote;

    #[tokio::test]
    async fn test_events_on_remove_liquidity_success() {
        let setup = setup_test_environment(true).await;
        let shares = 10_000;
        let quote_min_amount = 1;
        let base_min_amount = 1;

        let ix = remove_liquidity_instruction(
            &setup.program_id,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.lp_mint,
            shares,
            quote_min_amount,
            base_min_amount,
        );
        let mut tx = Transaction::new_with_payer(&[ix], Some(&setup.keypair.pubkey()));
        tx.sign(
            &[&setup.keypair],
            setup.client.get_latest_blockhash().await.unwrap(),
        );
        let result = setup.client.send_and_confirm_transaction(&tx).await;

        // Get the transaction details
        let tx = setup
            .client
            .get_transaction(
                &result.unwrap(),
                solana_transaction_status::UiTransactionEncoding::JsonParsed,
            )
            .await;
        let amm_acc = setup.client.get_account(&setup.amm_account).await.unwrap();
        let mut data = amm_acc.data.as_slice();
        let data = crate::state::Amm::try_deserialize(&mut data).unwrap();
        let (base_amount, quote_amount) = remove_quote(
            shares,
            data.base_reserve,
            data.quote_reserve,
            data.total_shares,
        )
        .unwrap();

        // Test the emitted events
        if let OptionSerializer::Some(logs) = tx.unwrap().transaction.meta.unwrap().log_messages {
            for log in logs {
                if log.starts_with("Program emit_cpi RemoveLiquidityEvent") {
                    let event_data: Vec<&str> = log.split_whitespace().collect();
                    assert_eq!(event_data[3], "base_amount:");
                    assert_eq!(event_data[4], base_amount.to_string());
                    assert_eq!(event_data[6], "quote_amount:");
                    assert_eq!(event_data[7], quote_amount.to_string());
                    assert_eq!(event_data[9], "shares:");
                    assert_eq!(event_data[10], shares.to_string());
                    // Add more assertions for other event fields as needed
                } else if log.starts_with("Program emit RemoveLiquidityEvent") {
                    let event_data: Vec<&str> = log.split_whitespace().collect();
                    assert_eq!(event_data[3], "base_amount:");
                    assert_eq!(event_data[4], base_amount.to_string());
                    assert_eq!(event_data[6], "quote_amount:");
                    assert_eq!(event_data[7], quote_amount.to_string());
                    assert_eq!(event_data[9], "shares:");
                    assert_eq!(event_data[10], shares.to_string());
                    // Add more assertions for other event fields as needed
                }
            }
        } else {
            panic!("No log messages found in the transaction metadata");
        }
    }

    #[tokio::test]
    async fn test_remove_liquidity_failure_insufficient_shares() {
        let setup = setup_test_environment(true).await;
        let shares = 0; // Deliberately low to trigger failure

        let ix = remove_liquidity_instruction(
            &setup.program_id,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.lp_mint,
            shares,
            1,
            1,
        );

        let mut tx = Transaction::new_with_payer(&[ix], Some(&setup.keypair.pubkey()));
        tx.sign(
            &[&setup.keypair],
            setup.client.get_latest_blockhash().await.unwrap(),
        );
        let result = setup.client.send_and_confirm_transaction(&tx).await;
        assert!(
            result.is_err(),
            "Transaction should fail due to insufficient shares"
        );
    }

    #[tokio::test]
    async fn test_remove_liquidity_updates_base_reserve_correctly() {
        let setup = setup_test_environment(true).await;
        let shares = 10_000;

        let initial_base_reserve = fetch_reserves(&setup).await.0;

        let ix = remove_liquidity_instruction(
            &setup.program_id,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.lp_mint,
            shares,
            1,
            1,
        );

        let mut tx = Transaction::new_with_payer(&[ix], Some(&setup.keypair.pubkey()));
        tx.sign(
            &[&setup.keypair],
            setup.client.get_latest_blockhash().await.unwrap(),
        );
        setup
            .client
            .send_and_confirm_transaction(&tx)
            .await
            .unwrap();

        let updated_base_reserve = fetch_reserves(&setup).await.0;

        assert_ne!(
            initial_base_reserve, updated_base_reserve,
            "Base reserve should be updated after remove liquidity transaction"
        );
    }

    #[tokio::test]
    async fn test_remove_liquidity_updates_quote_reserve_correctly() {
        let setup = setup_test_environment(true).await;
        let shares = 10_000;

        let initial_quote_reserve = fetch_reserves(&setup).await.1;

        let ix = remove_liquidity_instruction(
            &setup.program_id,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.lp_mint,
            shares,
            1,
            1,
        );

        let mut tx = Transaction::new_with_payer(&[ix], Some(&setup.keypair.pubkey()));
        tx.sign(
            &[&setup.keypair],
            setup.client.get_latest_blockhash().await.unwrap(),
        );
        setup
            .client
            .send_and_confirm_transaction(&tx)
            .await
            .unwrap();

        let updated_quote_reserve = fetch_reserves(&setup).await.1;

        assert_ne!(
            initial_quote_reserve, updated_quote_reserve,
            "Quote reserve should be updated after remove liquidity transaction"
        );
    }
}
