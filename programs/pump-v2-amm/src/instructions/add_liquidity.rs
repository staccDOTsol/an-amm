use crate::error::ErrorCode;
use crate::utils::{spl_token_transfer, TokenTransferParams};
use anchor_lang::prelude::*;
use num_integer::Roots;

// Define a constant for the minimum liquidity threshold
const MINIMUM_LIQUIDITY: u64 = 100_000;

// AddLiquidityEvent event
#[event]
pub struct AddLiquidityEvent {
    pub base_amount: u64,
    pub quote_amount: u64,
    pub shares: u64,
    pub timestamp: u64,
    pub user: Pubkey,
}

/// Calculates the number of shares a user will receive for their liquidity based on the
/// current state of the Automated Market Maker (AMM).
///
/// # Parameters:
/// - `base_token_amount`: Amount of the base token the user wants to deposit.
/// - `quote_token_amount`: Amount of the quote token the user wants to deposit.
/// - `base_reserve`: Current reserve of the base token in the AMM.
/// - `quote_reserve`: Current reserve of the quote token in the AMM.
/// - `total_shares`: Current total number of shares in the AMM.
///
/// # Returns:
/// - Result containing the number of shares or an error.
fn calculate_shares(
    base_token_amount: u64,
    quote_token_amount: u64,
    base_reserve: u64,
    quote_reserve: u64,
    total_shares: u64,
) -> Result<u64> {
    if total_shares != 0 {
        // Calculate amount of LP tokens as a fraction of existing reserves
        let base_token_share = (base_token_amount * total_shares) / base_reserve;
        let fractional_token_share = (quote_token_amount * total_shares) / quote_reserve;
        Ok(base_token_share.min(fractional_token_share))
    } else {
        // Initialize shares when there's no existing liquidity
        let initial_shares = (base_token_amount * quote_token_amount).sqrt();
        Ok(initial_shares)
    }
}

/// This submodule is dedicated to handling the addition of liquidity to the Automated Market Maker (AMM).
/// It includes functions and structures necessary for processing liquidity transactions, ensuring that
/// the liquidity added is correctly accounted for and integrated into the AMM's existing pool.
pub mod add_liquidity {
    use super::*;
    use crate::AddLiquidity;

    /// Handles liquidity addition to the AMM, ensuring that liquidity constraints are met and updating AMM reserves.
    ///
    /// # Parameters:
    /// - `ctx`: Context containing all the accounts required to execute the operation.
    /// - `base_amount`: Amount of the base currency being added.
    /// - `quote_amount`: Amount of the quote currency being added.
    /// - `min_lp_shares`: Minimum number of liquidity provider (LP) shares the user expects to receive.
    ///
    /// # Returns:
    /// - Result indicating success or error state.
    pub fn handler(
        ctx: Context<AddLiquidity>,
        base_amount: u64,
        quote_amount: u64,
        min_lp_shares: u64,
    ) -> Result<()> {
        // Retrieve account information for mints and token accounts
        let base_mint = &ctx.accounts.base_mint;
        let quote_mint = &ctx.accounts.quote_mint;
        let base_reserve_ata = &ctx.accounts.base_reserve_ata;
        let quote_reserve_ata = &ctx.accounts.quote_reserve_ata;
        let lp_mint = &ctx.accounts.lp_mint;

        // Calculate and mint liquidity tokens (shares)
        let shares = calculate_shares(
            base_amount,
            quote_amount,
            ctx.accounts.amm.base_reserve,
            ctx.accounts.amm.quote_reserve,
            ctx.accounts.amm.total_shares,
        )?;
        require_gte!(shares, min_lp_shares, ErrorCode::InsufficientLiquidity);
        // Update AMM state with new reserves and total shares
        let amm = &mut ctx.accounts.amm;
        amm.base_reserve += base_amount;
        amm.quote_reserve += quote_amount;
        amm.total_shares += shares;
        // Define signer seeds for transactions requiring signatures
        let signer_seeds = [
            b"amm",
            ctx.accounts.amm.creator.as_ref(),
            base_mint.to_account_info().key.as_ref(),
            quote_mint.to_account_info().key.as_ref(),
            &[ctx.bumps.amm],
        ];

        // Mint the minimum liquidity to the user
        anchor_spl::token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.base_token_program.to_account_info(),
                anchor_spl::token::MintTo {
                    mint: ctx.accounts.lp_mint.to_account_info(),
                    to: ctx.accounts.user_lp_ata.to_account_info(),
                    authority: ctx.accounts.amm.to_account_info(),
                },
                &[&signer_seeds],
            ),
            shares,
        )?;

        // Handle liquidity initialization case when LP mint amount is zero
        if lp_mint.supply == 0 {
            // Burn the minimum liquidity from the user's account
            anchor_spl::token::burn(
                CpiContext::new(
                    ctx.accounts.base_token_program.to_account_info(),
                    anchor_spl::token::Burn {
                        mint: ctx.accounts.lp_mint.to_account_info(),
                        from: ctx.accounts.user_lp_ata.to_account_info(),
                        authority: ctx.accounts.user.to_account_info(),
                    },
                ),
                MINIMUM_LIQUIDITY,
            )?;
        }

        // Transfer base tokens to the AMM reserves
        spl_token_transfer(TokenTransferParams {
            source: ctx.accounts.user_base_ata.to_account_info(),
            destination: base_reserve_ata.to_account_info(),
            amount: base_amount,
            authority: ctx.accounts.user.to_account_info(),
            authority_signer_seeds: &[],
            decimals: ctx.accounts.base_mint.decimals,
            mint: ctx.accounts.base_mint.to_account_info(),
            token_program: ctx.accounts.base_token_program.to_account_info(),
        })?;

        // Transfer quote tokens to the AMM reserves
        spl_token_transfer(TokenTransferParams {
            source: ctx.accounts.user_quote_ata.to_account_info(),
            destination: quote_reserve_ata.to_account_info(),
            amount: quote_amount,
            authority: ctx.accounts.user.to_account_info(),
            authority_signer_seeds: &[],
            decimals: ctx.accounts.quote_mint.decimals,
            mint: ctx.accounts.quote_mint.to_account_info(),
            token_program: ctx.accounts.quote_token_program.to_account_info(),
        })?;

        // Emit the event
        emit_cpi!(AddLiquidityEvent {
            base_amount,
            quote_amount,
            shares,
            timestamp: Clock::get()?.unix_timestamp as u64,
            user: *ctx.accounts.user.key,
        });

        emit!(AddLiquidityEvent {
            base_amount,
            quote_amount,
            shares,
            timestamp: Clock::get()?.unix_timestamp as u64,
            user: *ctx.accounts.user.key,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::fixtures::tests::fetch_reserves;
    use crate::fixtures::tests::setup_test_environment;
    use crate::instructions::add_liquidity::MINIMUM_LIQUIDITY;

    use crate::fixtures::tests::setup_amm;
    use crate::fixtures::tests::setup_mints_and_accounts;
    use crate::fixtures::tests::setup_user_accounts;
    use crate::instructions::add_liquidity::calculate_shares;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use solana_transaction_status::option_serializer::OptionSerializer;
    use std::str::FromStr;
    use std::sync::Arc;

    pub fn add_liquidity_instruction(
        program_id: &solana_sdk::pubkey::Pubkey,
        amm_pubkey: &solana_sdk::pubkey::Pubkey,
        base_pubkey: &solana_sdk::pubkey::Pubkey,
        quote_pubkey: &solana_sdk::pubkey::Pubkey,
        payer_pubkey: &solana_sdk::pubkey::Pubkey,
        lp_mint_pubkey: &solana_sdk::pubkey::Pubkey,
        base_amount: u64,
        quote_amount: u64,
        min_liquidity: u64,
    ) -> solana_sdk::instruction::Instruction {
        let mut data = switchboard_solana::get_ixn_discriminator("add_liquidity").to_vec();
        data.extend_from_slice(&base_amount.to_le_bytes());
        data.extend_from_slice(&quote_amount.to_le_bytes());
        data.extend_from_slice(&min_liquidity.to_le_bytes());
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
    async fn test_events_on_add_liquidity_success() {
        let setup = setup_test_environment(true).await;
        let base_amount = 100_000;
        let quote_amount = 100_000;
        let min_lp_shares = 1;

        let ix = add_liquidity_instruction(
            &setup.program_id,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.lp_mint,
            base_amount,
            quote_amount,
            min_lp_shares,
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

        // Test the emitted events
        if let OptionSerializer::Some(logs) = tx.unwrap().transaction.meta.unwrap().log_messages {
            for log in logs {
                if log.starts_with("Program emit_cpi AddLiquidityEvent") {
                    let event_data: Vec<&str> = log.split_whitespace().collect();
                    assert_eq!(event_data[3], "base_amount:");
                    assert_eq!(event_data[4], base_amount.to_string());
                    assert_eq!(event_data[6], "quote_amount:");
                    assert_eq!(event_data[7], quote_amount.to_string());
                    // Add more assertions for other event fields as needed
                } else if log.starts_with("Program emit AddLiquidityEvent") {
                    let event_data: Vec<&str> = log.split_whitespace().collect();
                    assert_eq!(event_data[3], "base_amount:");
                    assert_eq!(event_data[4], base_amount.to_string());
                    assert_eq!(event_data[6], "quote_amount:");
                    assert_eq!(event_data[7], quote_amount.to_string());
                    // Add more assertions for other event fields as needed
                }
            }
        } else {
            panic!("No log messages found in the transaction metadata");
        }
    }

    #[tokio::test]
    async fn test_add_liquidity_success() {
        // Execute the async test within the Tokio runtime
        let setup = setup_test_environment(false).await;
        let base_amount = 100000000;
        let quote_amount = 1000000000;
        let min_lp_shares = 1;

        let ix = add_liquidity_instruction(
            &setup.program_id,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.lp_mint,
            base_amount,
            quote_amount,
            min_lp_shares,
        );
        let mut tx = Transaction::new_with_payer(&vec![ix], Some(&setup.keypair.pubkey()));
        tx.sign(
            &[&setup.keypair],
            setup.client.get_latest_blockhash().await.unwrap(),
        );
        let result = setup.client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", result);
    }
    #[tokio::test]
    async fn test_add_liquidity_failure_insufficient_shares() {
        let setup = setup_test_environment(false).await;
        let base_amount = 1_000; // Normal amount
        let quote_amount = 1_000; // Normal amount
        let min_lp_shares = 1_000_000; // Unreasonably high minimum expectation

        let ix = add_liquidity_instruction(
            &setup.program_id,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.lp_mint,
            base_amount,
            quote_amount,
            min_lp_shares,
        );

        let mut tx = Transaction::new_with_payer(&vec![ix], Some(&setup.keypair.pubkey()));
        tx.sign(
            &[&setup.keypair],
            setup.client.get_latest_blockhash().await.unwrap(),
        );

        let result = setup.client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", result);
        assert!(
            result.is_err(),
            "Transaction should fail due to insufficient liquidity shares"
        );
    }

    #[tokio::test]
    async fn test_add_liquidity_edge_case_minimum_initial_liquidity() {
        let setup = setup_test_environment(false).await;
        let base_amount = 100_000; // Very small amount
        let quote_amount = 100_000; // Very small amount
        let min_lp_shares = 1; // Minimum possible shares

        let ix = add_liquidity_instruction(
            &setup.program_id,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.lp_mint,
            base_amount,
            quote_amount,
            min_lp_shares,
        );

        let mut tx = Transaction::new_with_payer(&vec![ix], Some(&setup.keypair.pubkey()));
        tx.sign(
            &[&setup.keypair],
            setup.client.get_latest_blockhash().await.unwrap(),
        );
        let result = setup.client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", result);
        assert!(
            result.is_ok(),
            "Transaction should succeed even with minimal initial liquidity"
        );
    }
    #[tokio::test]
    async fn test_add_liquidity_updates_base_reserve_correctly() {
        let setup = setup_test_environment(true).await;
        let base_amount = 100_000;
        let quote_amount = 100_000;

        let initial_base_reserve = fetch_reserves(&setup).await.0;

        let ix = add_liquidity_instruction(
            &setup.program_id,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.lp_mint,
            base_amount,
            quote_amount,
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
            "Base reserve should be updated after add liquidity transaction"
        );
    }
    #[tokio::test]
    async fn test_add_liquidity_updates_quote_reserve_correctly() {
        let setup = setup_test_environment(true).await;
        let base_amount = 100_000;
        let quote_amount = 100_000;

        let initial_quote_reserve = fetch_reserves(&setup).await.1;

        let ix = add_liquidity_instruction(
            &setup.program_id,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.lp_mint,
            base_amount,
            quote_amount,
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
            "Quote reserve should be updated after add liquidity transaction"
        );
    }
}
