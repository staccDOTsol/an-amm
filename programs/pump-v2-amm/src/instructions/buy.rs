use crate::error::ErrorCode;
use crate::utils::{spl_token_transfer, TokenTransferParams};
use crate::{Amm, Buy};
use anchor_lang::prelude::*;

// BuyEvent event
#[event]
pub struct BuyEvent {
    pub base_amount: u64,
    pub quote_amount: u64,
    pub timestamp: u64,
    pub user: Pubkey,
    pub protocol_fee_amount: u64,
    pub referrer: Option<Pubkey>,
    pub referrer_fee_amount: Option<u64>,
}

/// Calculates the required quote amount to obtain a specified amount of base tokens.
///
/// # Parameters:
/// - `output_amount`: Desired amount of base tokens.
/// - `amm`: Reference to the current state of the AMM.
///
/// # Returns:
/// - Result containing the calculated quote amount or an error.
fn buy_quote(output_amount: u64, amm: &Amm) -> Result<u64> {
    let input_amount = (output_amount * amm.quote_reserve) / (amm.base_reserve - output_amount);
    Ok(input_amount)
}

// This module is responsible for processing buy transactions where users exchange quote tokens for base tokens.
pub mod buy {
    use super::*;

    /// Executes a buy transaction where a user buys base tokens using quote tokens.
    ///
    /// # Parameters:
    /// - `ctx`: Context containing all required accounts for the transaction.
    /// - `base_amount`: The amount of base tokens the user wants to buy.
    /// - `max_quote_amount`: The maximum amount of quote tokens the user is willing to spend.
    ///
    /// # Returns:
    /// - Result indicating success or an error.
    pub fn handler<'a>(
        ctx: anchor_lang::context::Context<'_, '_, '_, 'a, Buy<'a>>,
        base_amount: u64,
        max_quote_amount: u64,
    ) -> Result<()> {
        let quote_token_program = ctx.accounts.quote_token_program.to_account_info();
        let base_token_program = ctx.accounts.base_token_program.to_account_info();

        // Calculate the required quote amount for the desired base amount
        let quote_amount = buy_quote(base_amount, &*ctx.accounts.amm)?;

        // Ensure the quote amount does not exceed what the user is willing to pay
        require_gte!(
            max_quote_amount,
            quote_amount,
            ErrorCode::InsufficientQuoteAmount
        );

        // Decrease base reserve and increase quote reserve by the transaction amounts
        let amm = &mut ctx.accounts.amm;
        amm.base_reserve -= base_amount;
        amm.quote_reserve += quote_amount;

        // Define seeds for signing transactions involving the AMM
        let signer_seeds = [
            b"amm",
            ctx.accounts.amm.creator.as_ref(),
            ctx.accounts.base_mint.to_account_info().key.as_ref(),
            ctx.accounts.quote_mint.to_account_info().key.as_ref(),
            &[ctx.bumps.amm],
        ];

        let mut protocol_fee_bps = ctx.accounts.global_parameters.protocol_fee_bps;
        let mut referrer_fee_amount = None;
        // Transfer the fee to refferer if set and apply protocol fee discount.
        if ctx.remaining_accounts.len() > 0 {
            protocol_fee_bps -= ctx.accounts.global_parameters.referrer_fee_discount_bps;
            let referrer_fee_bps = ctx.accounts.global_parameters.referrer_fee_bps;
            referrer_fee_amount = Some((quote_amount * referrer_fee_bps) / 10000);

            spl_token_transfer(TokenTransferParams {
                source: ctx.accounts.user_quote_ata.to_account_info(),
                // This account is checked in one of the token program instructions
                destination: ctx.remaining_accounts.get(0).unwrap().to_account_info(),
                amount: referrer_fee_amount.unwrap(),
                authority: ctx.accounts.user.to_account_info(),
                authority_signer_seeds: &[],
                decimals: ctx.accounts.quote_mint.decimals,
                mint: ctx.accounts.quote_mint.to_account_info(),
                token_program: quote_token_program.clone(),
            })?;
        }

        // Transfer the protocol fee to the fee receiver
        let protocol_fee_amount = (quote_amount * protocol_fee_bps) / 10000;
        spl_token_transfer(TokenTransferParams {
            source: ctx.accounts.user_quote_ata.to_account_info(),
            destination: ctx.accounts.fee_receiver_ata.to_account_info(),
            amount: protocol_fee_amount,
            authority: ctx.accounts.user.to_account_info(),
            authority_signer_seeds: &[],
            decimals: ctx.accounts.quote_mint.decimals,
            mint: ctx.accounts.quote_mint.to_account_info(),
            token_program: quote_token_program.clone(),
        })?;

        // Transfer the quote amount to the AMM reserves
        spl_token_transfer(TokenTransferParams {
            source: ctx.accounts.user_quote_ata.to_account_info(),
            destination: ctx.accounts.quote_reserve_ata.to_account_info(),
            amount: quote_amount,
            authority: ctx.accounts.user.to_account_info(),
            authority_signer_seeds: &signer_seeds,
            decimals: ctx.accounts.quote_mint.decimals,
            mint: ctx.accounts.quote_mint.to_account_info(),
            token_program: quote_token_program.clone(),
        })?;

        // Transfer the purchased base amount to the user
        spl_token_transfer(TokenTransferParams {
            source: ctx.accounts.base_reserve_ata.to_account_info(),
            destination: ctx.accounts.user_base_ata.to_account_info(),
            amount: base_amount,
            authority: ctx.accounts.amm.to_account_info(),
            authority_signer_seeds: &signer_seeds,
            decimals: ctx.accounts.base_mint.decimals,
            mint: ctx.accounts.base_mint.to_account_info(),
            token_program: base_token_program.clone(),
        })?;

        // Emit the event
        emit_cpi!(BuyEvent {
            base_amount,
            quote_amount,
            user: *ctx.accounts.user.to_account_info().key,
            timestamp: Clock::get()?.unix_timestamp as u64,
            referrer: ctx
                .remaining_accounts
                .get(0)
                .map(|r| *r.to_account_info().key),
            referrer_fee_amount,
            protocol_fee_amount
        });

        emit!(BuyEvent {
            base_amount,
            quote_amount,
            user: *ctx.accounts.user.to_account_info().key,
            timestamp: Clock::get()?.unix_timestamp as u64,
            referrer: ctx
                .remaining_accounts
                .get(0)
                .map(|r| *r.to_account_info().key),
            referrer_fee_amount,
            protocol_fee_amount
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
    use crate::fixtures::tests::TestEnvironment;
    use crate::Pubkey;
    use std::str::FromStr;

    use anchor_lang::AccountDeserialize;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_sdk::program_pack::Pack;
    use solana_sdk::{
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use solana_transaction_status::option_serializer::OptionSerializer;
    use std::sync::Arc;

    fn buy_instruction(
        program_id: &solana_sdk::pubkey::Pubkey,
        global_parameters: &solana_sdk::pubkey::Pubkey,
        amm_pubkey: &solana_sdk::pubkey::Pubkey,
        base_pubkey: &solana_sdk::pubkey::Pubkey,
        quote_pubkey: &solana_sdk::pubkey::Pubkey,
        payer_pubkey: &solana_sdk::pubkey::Pubkey,
        referrer_pubkey: &solana_sdk::pubkey::Pubkey,
        base_amount: u64,
        max_quote_amount: u64,
    ) -> solana_sdk::instruction::Instruction {
        let mut data = switchboard_solana::get_ixn_discriminator("buy").to_vec();
        data.extend_from_slice(&base_amount.to_le_bytes());
        data.extend_from_slice(&max_quote_amount.to_le_bytes());
        // Construct the accounts required for the buy_instruction
        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(*amm_pubkey, false),
            solana_sdk::instruction::AccountMeta::new(*global_parameters, false),
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
                    &quote_pubkey,
                    &spl_token::ID,
                ),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new(*base_pubkey, false),
            solana_sdk::instruction::AccountMeta::new(*quote_pubkey, false),
            solana_sdk::instruction::AccountMeta::new_readonly(spl_token_2022::ID, false),
            solana_sdk::instruction::AccountMeta::new_readonly(spl_token::ID, false),
            solana_sdk::instruction::AccountMeta::new_readonly(
                Pubkey::from_str("38C9cb9ak6zRdtA3ZxKPp9sYAPEKT9KfZcUcdC5Tda69").unwrap(),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new_readonly(*program_id, false),
            solana_sdk::instruction::AccountMeta::new(
                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &referrer_pubkey,
                    &quote_pubkey,
                    &spl_token::ID,
                ),
                false,
            ),
        ];
        // Create the instruction using the program_id, accounts, and data
        solana_sdk::instruction::Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    }
    #[tokio::test]
    async fn test_buy_success() {
        let setup = setup_test_environment(true).await;
        let base_amount = 100_000;
        let max_quote_amount = 100100100;

        let ix = buy_instruction(
            &setup.program_id,
            &setup.global_parameters,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.keypair.pubkey(),
            base_amount,
            max_quote_amount,
        );

        let mut tx = Transaction::new_with_payer(&[ix], Some(&setup.keypair.pubkey()));
        tx.sign(
            &[&setup.keypair],
            setup.client.get_latest_blockhash().await.unwrap(),
        );
        let result = setup.client.send_and_confirm_transaction(&tx).await;
        println!("{:?}", result);
        assert!(result.is_ok(), "Buy transaction should succeed");
    }
    #[tokio::test]
    async fn test_events_on_success() {
        let setup = setup_test_environment(true).await;
        let base_amount = 100_000;
        let max_quote_amount = 100100100;
        let ix = buy_instruction(
            &setup.program_id,
            &setup.global_parameters,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.keypair.pubkey(),
            base_amount,
            max_quote_amount,
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
                if log.starts_with("Program emit_cpi BuyEvent") {
                    let event_data: Vec<&str> = log.split_whitespace().collect();
                    assert_eq!(event_data[3], "base_amount:");
                    assert_eq!(event_data[4], base_amount.to_string());
                    // Add more assertions for other event fields as needed
                } else if log.starts_with("Program emit BuyEvent") {
                    let event_data: Vec<&str> = log.split_whitespace().collect();
                    assert_eq!(event_data[3], "base_amount:");
                    assert_eq!(event_data[4], base_amount.to_string());
                    // Add more assertions for other event fields as needed
                }
            }
        } else {
            panic!("No log messages found in the transaction metadata");
        }
    }
    #[tokio::test]
    async fn test_buy_failure_insufficient_quote() {
        let setup = setup_test_environment(true).await;
        let base_amount = 100_000;
        let max_quote_amount = 10; // Deliberately low to trigger failure

        let ix = buy_instruction(
            &setup.program_id,
            &setup.global_parameters,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.keypair.pubkey(),
            base_amount,
            max_quote_amount,
        );

        let mut tx = Transaction::new_with_payer(&[ix], Some(&setup.keypair.pubkey()));
        tx.sign(
            &[&setup.keypair],
            setup.client.get_latest_blockhash().await.unwrap(),
        );
        let result = setup.client.send_and_confirm_transaction(&tx).await;
        assert!(
            result.is_err(),
            "Transaction should fail due to insufficient quote amount"
        );
    }
    #[tokio::test]
    async fn test_buy_updates_base_reserve_correctly() {
        let setup = setup_test_environment(true).await;
        let base_amount = 100_000;
        let max_quote_amount = 1001001;

        // Fetch initial base reserve
        let initial_base_reserve = fetch_reserves(&setup).await.0;

        let ix = buy_instruction(
            &setup.program_id,
            &setup.global_parameters,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.keypair.pubkey(),
            base_amount,
            max_quote_amount,
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

        // Fetch updated base reserve
        let updated_base_reserve = fetch_reserves(&setup).await.0;

        assert_ne!(
            initial_base_reserve, updated_base_reserve,
            "Base reserve should be updated after buy transaction"
        );
    }

    #[tokio::test]
    async fn test_buy_updates_quote_reserve_correctly() {
        let setup = setup_test_environment(true).await;
        let base_amount = 100_000;
        let max_quote_amount = 1001001;

        // Fetch initial quote reserve
        let initial_quote_reserve = fetch_reserves(&setup).await.1;

        let ix = buy_instruction(
            &setup.program_id,
            &setup.global_parameters,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.keypair.pubkey(),
            base_amount,
            max_quote_amount,
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

        // Fetch updated quote reserve
        let updated_quote_reserve = fetch_reserves(&setup).await.1;

        assert_ne!(
            initial_quote_reserve, updated_quote_reserve,
            "Quote reserve should be updated after buy transaction"
        );
    }

    #[tokio::test]
    async fn test_buy_updates_fee_receiver_balance_correctly() {
        let setup = setup_test_environment(true).await;
        let base_amount = 100_000;
        let max_quote_amount = 1001001;

        // Fetch initial balance of fee receiver
        let initial_fee_receiver_balance = setup
            .client
            .get_token_account_balance(
                &spl_associated_token_account::get_associated_token_address_with_program_id(
                    &setup.keypair.pubkey(),
                    &setup.quote_mint,
                    &spl_token::ID,
                ),
            )
            .await
            .unwrap()
            .amount
            .parse::<u64>()
            .unwrap();

        let ix = buy_instruction(
            &setup.program_id,
            &setup.global_parameters,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &setup.keypair.pubkey(),
            base_amount,
            max_quote_amount,
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

        // Fetch updated balance of fee receiver
        let updated_fee_receiver_balance = setup
            .client
            .get_token_account_balance(
                &spl_associated_token_account::get_associated_token_address_with_program_id(
                    &setup.keypair.pubkey(),
                    &setup.quote_mint,
                    &spl_token::ID,
                ),
            )
            .await
            .unwrap()
            .amount
            .parse::<u64>()
            .unwrap();

        assert_ne!(
            initial_fee_receiver_balance, updated_fee_receiver_balance,
            "Fee receiver's balance should be updated after buy transaction"
        );
    }

    #[tokio::test]
    async fn test_buy_with_referrer() {
        let setup = setup_test_environment(true).await;
        let base_amount = 100_000;
        let max_quote_amount = 1001001;

        // Create a new keypair for the referrer
        let referrer_keypair = Keypair::new();

        // Create an associated token account for the referrer with the setup keypair as the payer
        let referrer_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &setup.keypair.pubkey(),
                &referrer_keypair.pubkey(),
                &setup.quote_mint,
                &spl_token::ID,
            );

        // Use the referrer's pubkey in the buy instruction
        let ix = buy_instruction(
            &setup.program_id,
            &setup.global_parameters,
            &setup.amm_account,
            &setup.base_mint,
            &setup.quote_mint,
            &setup.keypair.pubkey(),
            &referrer_keypair.pubkey(),
            base_amount,
            max_quote_amount,
        );

        let mut tx =
            Transaction::new_with_payer(&[referrer_ata_ix, ix], Some(&setup.keypair.pubkey()));
        tx.sign(
            &[&setup.keypair],
            setup.client.get_latest_blockhash().await.unwrap(),
        );
        let result = setup.client.send_and_confirm_transaction(&tx).await;
        assert!(
            result.is_ok(),
            "Buy transaction with referrer should succeed"
        );

        // Fetch the balance of the referrer's associated token account after the transaction
        let referrer_ata_balance = setup
            .client
            .get_token_account_balance(&spl_associated_token_account::get_associated_token_address(
                &referrer_keypair.pubkey(),
                &setup.quote_mint,
            ))
            .await
            .unwrap();
        // Calculate expected referrer fee amount
        let global_parameters_info = setup
            .client
            .get_account(&setup.global_parameters)
            .await
            .unwrap();
        let mut data: &[u8] = &global_parameters_info.data;
        let global_parameters = crate::state::GlobalParameters::try_deserialize(&mut data).unwrap();
        let referrer_fee_bps = global_parameters.referrer_fee_bps;
        let expected_referrer_fee = (max_quote_amount * referrer_fee_bps) / 10000;

        // Check if the referrer's ATA balance has increased by the expected referrer fee amount
        assert_eq!(
            referrer_ata_balance.amount.parse::<u64>().unwrap(),
            expected_referrer_fee,
            "Referrer's token account balance should increase by the expected referrer fee amount"
        );
    }
}
