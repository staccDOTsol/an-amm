#[cfg(test)]
pub mod tests {
    use anchor_spl::token::{self, Mint};
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_sdk::pubkey::Pubkey;
    use solana_sdk::signature::Keypair;
    use solana_sdk::signature::Signer;
    use solana_sdk::transaction::Transaction;
    use std::str::FromStr;
    use std::sync::Arc;

    pub async fn setup_mints_and_accounts(
        client: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
        payer: &Arc<Keypair>,
    ) -> Result<(Pubkey, Pubkey), Box<dyn std::error::Error>> {
        let base_mint = Keypair::new();
        let quote_mint = Keypair::new();

        // Initialize mints and create associated token accounts
        let create_account_x_ix = solana_program::system_instruction::create_account(
            &payer.pubkey(),
            &base_mint.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Mint::LEN)
                .await
                .unwrap(),
            Mint::LEN as u64,
            &spl_token_2022::ID,
        );
        let init_base_mint_ix = spl_token_2022::instruction::initialize_mint(
            &spl_token_2022::ID,
            &base_mint.pubkey(),
            &payer.pubkey(),
            None,
            9,
        )?;

        let create_account_y_ix = solana_program::system_instruction::create_account(
            &payer.pubkey(),
            &quote_mint.pubkey(),
            client
                .get_minimum_balance_for_rent_exemption(Mint::LEN)
                .await
                .unwrap(),
            Mint::LEN as u64,
            &spl_token::ID,
        );
        let init_quote_mint_ix = spl_token::instruction::initialize_mint(
            &token::ID,
            &quote_mint.pubkey(),
            &payer.pubkey(),
            None,
            9,
        )?;

        let mut create_and_init_mints_tx = Transaction::new_with_payer(
            &[
                create_account_x_ix,
                create_account_y_ix,
                init_base_mint_ix,
                init_quote_mint_ix,
            ],
            Some(&payer.pubkey()),
        );
        create_and_init_mints_tx.sign(
            &[payer, &base_mint, &quote_mint],
            client.get_latest_blockhash().await?,
        );
        client
            .send_and_confirm_transaction(&create_and_init_mints_tx)
            .await?;
        println!("Mints created and initialized");

        Ok((base_mint.pubkey(), quote_mint.pubkey()))
    }

    fn set_parameters_instruction(
        program_id: &solana_sdk::pubkey::Pubkey,
        global_parameters: &solana_sdk::pubkey::Pubkey,
        payer_pubkey: &solana_sdk::pubkey::Pubkey,
    ) -> solana_sdk::instruction::Instruction {
        let mut data = switchboard_solana::get_ixn_discriminator("set_parameters").to_vec();
        data.extend_from_slice(&100u64.to_le_bytes());
        data.extend_from_slice(&10u64.to_le_bytes());
        data.extend_from_slice(&20u64.to_le_bytes());
        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(*global_parameters, false),
            solana_sdk::instruction::AccountMeta::new_readonly(*payer_pubkey, true),
            solana_sdk::instruction::AccountMeta::new_readonly(
                Pubkey::from_str("38C9cb9ak6zRdtA3ZxKPp9sYAPEKT9KfZcUcdC5Tda69").unwrap(),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new_readonly(*program_id, false),
        ];
        solana_sdk::instruction::Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    }
    fn initialize_instruction(
        program_id: &solana_sdk::pubkey::Pubkey,
        payer_pubkey: &solana_sdk::pubkey::Pubkey,
        global_parameters: &solana_sdk::pubkey::Pubkey,
    ) -> solana_sdk::instruction::Instruction {
        let data = switchboard_solana::get_ixn_discriminator("initialize").to_vec();
        let accounts = vec![
            solana_sdk::instruction::AccountMeta::new(*payer_pubkey, true),
            solana_sdk::instruction::AccountMeta::new(*global_parameters, false),
            solana_sdk::instruction::AccountMeta::new(solana_program::system_program::ID, false),
            solana_sdk::instruction::AccountMeta::new_readonly(
                Pubkey::from_str("38C9cb9ak6zRdtA3ZxKPp9sYAPEKT9KfZcUcdC5Tda69").unwrap(),
                false,
            ),
            solana_sdk::instruction::AccountMeta::new_readonly(*program_id, false),
        ];
        solana_sdk::instruction::Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    }

    fn buy_instruction(
        program_id: &solana_sdk::pubkey::Pubkey,
        global_parameters: &solana_sdk::pubkey::Pubkey,
        amm_pubkey: &solana_sdk::pubkey::Pubkey,
        base_pubkey: &solana_sdk::pubkey::Pubkey,
        quote_pubkey: &solana_sdk::pubkey::Pubkey,
        payer_pubkey: &solana_sdk::pubkey::Pubkey,
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
            solana_sdk::instruction::AccountMeta::new(
                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &payer_pubkey,
                    &quote_pubkey,
                    &spl_token::ID,
                ),
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
    fn sell_instruction(
        program_id: &solana_sdk::pubkey::Pubkey,
        global_parameters: &solana_sdk::pubkey::Pubkey,
        amm_pubkey: &solana_sdk::pubkey::Pubkey,
        payer_pubkey: &solana_sdk::pubkey::Pubkey,
        base_pubkey: &solana_sdk::pubkey::Pubkey,
        quote_pubkey: &solana_sdk::pubkey::Pubkey,
        base_amount: u64,
        min_quote_amount: u64,
    ) -> solana_sdk::instruction::Instruction {
        let mut data = switchboard_solana::get_ixn_discriminator("sell").to_vec();
        data.extend_from_slice(&base_amount.to_le_bytes());
        data.extend_from_slice(&min_quote_amount.to_le_bytes());

        // Construct the accounts required for the add_liquidity_instruction
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
            solana_sdk::instruction::AccountMeta::new(
                spl_associated_token_account::get_associated_token_address_with_program_id(
                    &payer_pubkey,
                    &quote_pubkey,
                    &spl_token::ID,
                ),
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

    fn create_amm_instruction(
        program_id: &solana_sdk::pubkey::Pubkey,
        amm_pubkey: &solana_sdk::pubkey::Pubkey,
        base_pubkey: &solana_sdk::pubkey::Pubkey,
        quote_pubkey: &solana_sdk::pubkey::Pubkey,
        payer_pubkey: &solana_sdk::pubkey::Pubkey,
        lp_mint_pubkey: &solana_sdk::pubkey::Pubkey,
        fee_recipient_pubkey: &solana_sdk::pubkey::Pubkey,
    ) -> solana_sdk::instruction::Instruction {
        // Construct the data payload according to the program's expected schema
        let data = switchboard_solana::get_ixn_discriminator("create").to_vec();
        solana_sdk::instruction::Instruction {
            program_id: *program_id,
            accounts: vec![
                solana_sdk::instruction::AccountMeta::new(*amm_pubkey, false),
                solana_sdk::instruction::AccountMeta::new(*payer_pubkey, true),
                solana_sdk::instruction::AccountMeta::new_readonly(
                    solana_program::system_program::ID,
                    false,
                ),
                solana_sdk::instruction::AccountMeta::new(*lp_mint_pubkey, true),
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
                        &fee_recipient_pubkey,
                        &quote_pubkey,
                        &spl_token::ID,
                    ),
                    false,
                ),
                solana_sdk::instruction::AccountMeta::new_readonly(*base_pubkey, false),
                solana_sdk::instruction::AccountMeta::new_readonly(*quote_pubkey, false),
                solana_sdk::instruction::AccountMeta::new_readonly(spl_token_2022::ID, false),
                solana_sdk::instruction::AccountMeta::new_readonly(spl_token::ID, false),
                solana_sdk::instruction::AccountMeta::new_readonly(spl_token::ID, false),
                solana_sdk::instruction::AccountMeta::new_readonly(
                    Pubkey::from_str("SysvarRent111111111111111111111111111111111").unwrap(),
                    false,
                ),
                solana_sdk::instruction::AccountMeta::new_readonly(
                    Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap(),
                    false,
                ),
                solana_sdk::instruction::AccountMeta::new_readonly(
                    Pubkey::from_str("38C9cb9ak6zRdtA3ZxKPp9sYAPEKT9KfZcUcdC5Tda69").unwrap(),
                    false,
                ),
                solana_sdk::instruction::AccountMeta::new_readonly(*program_id, false),
            ],
            data,
        }
    }

    #[derive(Clone)]
    pub struct TestEnvironment {
        pub program_id: solana_sdk::pubkey::Pubkey,
        pub client: Arc<RpcClient>,
        pub keypair: Arc<Keypair>,
        pub amm_account: solana_sdk::pubkey::Pubkey,
        pub global_parameters: solana_sdk::pubkey::Pubkey,
        pub base_mint: solana_sdk::pubkey::Pubkey,
        pub quote_mint: solana_sdk::pubkey::Pubkey,
        pub lp_mint: solana_sdk::pubkey::Pubkey,
        pub user_base: solana_sdk::pubkey::Pubkey,
        pub user_quote: solana_sdk::pubkey::Pubkey,
        pub user_lp: solana_sdk::pubkey::Pubkey,
        pub base_vault: solana_sdk::pubkey::Pubkey,
        pub quote_vault: solana_sdk::pubkey::Pubkey,
    }

    pub async fn setup_test_environment(add_liquidity: bool) -> TestEnvironment {
        let keypair_bytes = [
            255, 201, 130, 141, 182, 122, 81, 76, 67, 90, 33, 58, 98, 198, 235, 188, 89, 218, 69,
            29, 52, 31, 65, 118, 181, 154, 208, 191, 136, 77, 137, 99, 229, 44, 114, 184, 249, 146,
            168, 186, 142, 70, 221, 54, 13, 200, 207, 148, 11, 6, 42, 28, 247, 26, 254, 58, 198,
            11, 40, 107, 96, 245, 235, 250,
        ];
        let keypair = Keypair::from_bytes(&keypair_bytes).unwrap();
        let keypair = Arc::new(keypair);
        let program_id = crate::id();
        let client = Arc::new(RpcClient::new(
            "https://jarrett-devnet-8fa6.devnet.rpcpool.com/283aba57-34a4-4500-ba4d-1832ff9ca64a"
                .to_string(),
        ));
        let (base_mint, quote_mint) = setup_mints_and_accounts(client.clone(), &keypair.clone())
            .await
            .unwrap();
        let (amm_account, global_parameters, lp_mint) = setup_amm(
            client.clone(),
            &keypair.clone(),
            &program_id,
            &base_mint,
            &quote_mint,
        )
        .await
        .unwrap();
        let (user_base, user_quote, user_lp, base_vault, quote_vault) = setup_user_accounts(
            client.clone(),
            &amm_account,
            &base_mint,
            &quote_mint,
            &lp_mint,
            &keypair.clone(),
        )
        .await
        .unwrap();

        if add_liquidity {
            let base_amount = 100000000;
            let quote_amount = 1000000000;
            let min_lp_shares = 1;
            let ix = add_liquidity_instruction(
                &program_id,
                &amm_account,
                &base_mint,
                &quote_mint,
                &keypair.pubkey(),
                &lp_mint,
                base_amount,
                quote_amount,
                min_lp_shares,
            );
            let mut tx = Transaction::new_with_payer(&[ix], Some(&keypair.pubkey()));
            tx.sign(&[&keypair], client.get_latest_blockhash().await.unwrap());
            client.send_and_confirm_transaction(&tx).await.unwrap();
        }

        TestEnvironment {
            program_id,
            client,
            keypair,
            amm_account,
            global_parameters,
            base_mint,
            quote_mint,
            lp_mint,
            user_base,
            user_quote,
            user_lp,
            base_vault,
            quote_vault,
        }
    }
    pub async fn setup_user_accounts(
        client: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
        amm_account: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        lp_mint: &Pubkey,
        keypair: &Arc<Keypair>,
    ) -> Result<
        (
            switchboard_solana::Pubkey,
            switchboard_solana::Pubkey,
            switchboard_solana::Pubkey,
            switchboard_solana::Pubkey,
            switchboard_solana::Pubkey,
        ),
        Box<dyn std::error::Error>,
    > {
        let user_authority = keypair.pubkey();
        let token_program = spl_token::ID;
        let token_2022_program = spl_token_2022::ID;

        // Create associated token accounts for the user
        let create_ata_base_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &user_authority,
                &user_authority,
                &base_mint,
                &token_2022_program,
            );
        let create_ata_lp_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &user_authority,
                &user_authority,
                &lp_mint,
                &token_program,
            );

        let mint_base_ix = spl_token_2022::instruction::mint_to(
            &spl_token_2022::ID,
            &base_mint,
            &spl_associated_token_account::get_associated_token_address_with_program_id(
                &user_authority,
                &base_mint,
                &spl_token_2022::ID,
            ),
            &user_authority,
            &[],
            10000000000000000,
        )?;

        let mint_quote_ix = spl_token::instruction::mint_to(
            &spl_token::ID,
            &quote_mint,
            &spl_associated_token_account::get_associated_token_address(
                &user_authority,
                &quote_mint,
            ),
            &user_authority,
            &[],
            10000000000000000,
        )?;
        let mut transaction = Transaction::new_with_payer(
            &[
                create_ata_lp_ix,
                create_ata_base_ix,
                mint_base_ix,
                mint_quote_ix,
            ],
            Some(&user_authority),
        );
        transaction.sign(&[&keypair], client.get_latest_blockhash().await?);
        client.send_and_confirm_transaction(&transaction).await?;
        println!("User accounts created");
        Ok((
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &user_authority,
                &base_mint,
                &token_2022_program,
            ),
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &user_authority,
                &quote_mint,
                &token_program,
            ),
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &user_authority,
                &lp_mint,
                &token_program,
            ),
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &amm_account,
                &base_mint,
                &token_2022_program,
            ),
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &amm_account,
                &quote_mint,
                &token_program,
            ),
        ))
    }

    pub async fn fetch_reserves(setup: &TestEnvironment) -> (u64, u64) {
        let rpc_client = &setup.client;
        let base_reserve = rpc_client
            .get_token_account_balance(&setup.base_vault)
            .await
            .unwrap();
        let quote_reserve = rpc_client
            .get_token_account_balance(&setup.quote_vault)
            .await
            .unwrap();
        (
            base_reserve.amount.parse::<u64>().unwrap(),
            quote_reserve.amount.parse::<u64>().unwrap(),
        )
    }
    pub async fn setup_amm(
        client: Arc<solana_client::nonblocking::rpc_client::RpcClient>,
        payer: &Arc<Keypair>,
        program_id: &Pubkey,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
    ) -> Result<(Pubkey, Pubkey, Pubkey), Box<dyn std::error::Error>> {
        let lp_mint = Keypair::new();
        let (global_parameters, _global_parameters_bump) =
            solana_sdk::pubkey::Pubkey::find_program_address(&[b"global_parameters"], &program_id);
        let (amm_account, _amm_account_bump) = Pubkey::find_program_address(
            &[
                b"amm",
                payer.pubkey().as_ref(),
                base_mint.as_ref(),
                quote_mint.as_ref(),
            ],
            program_id,
        );

        let global_parameters_account_info_maybe = client.get_account(&global_parameters).await;
        let mut ixs = vec![];
        println!(
            "global_parameters_account_info_maybe: {:?}",
            global_parameters_account_info_maybe
        );
        if !global_parameters_account_info_maybe.is_ok() {
            let initialize_instruction =
                initialize_instruction(&program_id, &payer.pubkey(), &global_parameters);

            let set_parameters_instruction =
                set_parameters_instruction(&program_id, &global_parameters, &payer.pubkey());
            ixs.push(initialize_instruction);
            ixs.push(set_parameters_instruction);
        }

        // Create associated token accounts for the vaults
        let create_ata_base_vault_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &payer.pubkey(),
                &amm_account,
                &base_mint,
                &spl_token_2022::ID,
            );
        let create_ata_quote_vault_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &payer.pubkey(),
                &amm_account,
                &quote_mint,
                &spl_token::ID,
            );
        let create_fee_recipient_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &payer.pubkey(),
                &payer.pubkey(),
                &quote_mint,
                &spl_token::ID,
            );
        ixs.push(create_fee_recipient_ix);
        let create_amm_instruction = create_amm_instruction(
            &program_id,
            &amm_account,
            &base_mint,
            &quote_mint,
            &payer.pubkey(),
            &lp_mint.pubkey(),
            &payer.pubkey(),
        );
        ixs.push(create_amm_instruction);
        let mut transaction = Transaction::new_with_payer(&ixs, Some(&payer.pubkey()));

        transaction.sign(&[payer, &lp_mint], client.get_latest_blockhash().await?);
        client.send_and_confirm_transaction(&transaction).await?;
        println!("AMM created");
        Ok((amm_account, global_parameters, lp_mint.pubkey()))
    }
}
