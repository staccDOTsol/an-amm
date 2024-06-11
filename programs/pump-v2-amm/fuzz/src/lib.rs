pub use account_state::{AccountInfoCache, AccountsState};
use anchor_lang::prelude::*;
use anchor_lang::{
    prelude::{
        Account, AccountInfo, AccountLoader, Context, Program, Pubkey, Rent, Signer, SolanaSysvar,
        Sysvar,
    },
    Discriminator, Key,
};
use fixed_macro::types::I80F48;
use pump_v2_amm::*;
use std::str::FromStr;

use solana_program::system_program;

use std::{
    collections::{BTreeMap, HashMap},
    mem::size_of,
    ops::AddAssign,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

pub mod account_state;
type SplAccount = spl_token::state::Account;
pub struct PumpAmmFuzzContext<'a> {
    pub owner: &'a AccountInfo<'a>,
    pub system_program: AccountInfo<'a>,
    pub token_program: AccountInfo<'a>,
    pub state: &'a AccountsState,
    pub base_mint: &'a AccountInfo<'a>,
    pub quote_mint: &'a AccountInfo<'a>,
    pub lp_mint: &'a AccountInfo<'a>,
    pub program_id: Pubkey,
    pub global_parameters: AccountInfo<'a>,
    pub global_parameters_bump: u8,
    pub amm: AccountInfo<'a>,
    pub amm_account_bump: u8,
    pub event_authority: Pubkey,
}
impl<'bump> PumpAmmFuzzContext<'bump> {
    pub fn get_amm_account_pubkey(
        owner: &AccountInfo<'bump>,
        base_mint: &AccountInfo<'bump>,
        quote_mint: &AccountInfo<'bump>,
    ) -> Pubkey {
        let (amm_account_key, _bump_seed) = Pubkey::find_program_address(
            &[
                b"amm",
                owner.key.as_ref(),
                base_mint.key.as_ref(),
                quote_mint.key.as_ref(),
            ],
            &crate::ID,
        );
        amm_account_key
    }

    pub fn setup_amm_account<'a: 'bump>(
        &'a self,
        owner: &'bump AccountInfo<'bump>,
        base_mint: &'bump AccountInfo<'bump>,
        quote_mint: &'bump AccountInfo<'bump>,
        lp_mint: &'bump AccountInfo<'bump>,
        amm_bump: u8,
        amm: &'bump AccountInfo<'bump>,
    ) {
        let amm_account_bump = self.amm_account_bump;
        let event_authority = self.state.new_event_authority(&self.event_authority);

        crate::account_state::set_discriminator::<crate::state::Amm>(amm.clone());

        pump_v2_amm::solana_amm::create(Context::new(
            &pump_v2_amm::id(),
            &mut crate::state::Create {
                amm: Account::try_from(&amm).unwrap(),
                user: Signer::try_from(&owner).unwrap(),
                system_program: Program::try_from(&self.system_program).unwrap(),
                lp_mint: InterfaceAccount::try_from(&lp_mint).unwrap(),
                base_mint: InterfaceAccount::try_from(&base_mint).unwrap(),
                quote_mint: InterfaceAccount::try_from(&quote_mint).unwrap(),
                token_program: Program::try_from(&self.token_program).unwrap(),
                //event_authority: event_authority,
                // program: self.state.new_program(crate::id()),
            },
            &[amm.clone()],
            pump_v2_amm::state::CreateBumps {
                amm: amm_bump,
                //event_authority: *self.state.bump.alloc(0),
            },
        ))
        .unwrap();
    }
    pub fn get_global_parameters(program_id: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"global_parameters"], &program_id)
    }
    pub fn get_program_id() -> Pubkey {
        crate::id()
    }
    // Main setup function
    pub fn setup(
        state: &'bump AccountsState,
        lp_mint: &'bump AccountInfo<'bump>,
        base_mint: &'bump AccountInfo<'bump>,
        quote_mint: &'bump AccountInfo<'bump>,
        amm_account_pubkey: &'bump Pubkey,
        amm_account_bump: u8,
        global_parameters_pubkey: &'bump Pubkey,
        global_parameters_bump: u8,
        owner: &'bump AccountInfo<'bump>,
    ) -> Self {
        let program_id = PumpAmmFuzzContext::get_program_id();

        let system_program = PumpAmmFuzzContext::setup_system_program(state);
        let token_program = PumpAmmFuzzContext::setup_token_program(state);

        let (amm, amm_account_bump) = state.new_amm_account(
            *owner.key,
            Rent::free(),
            &amm_account_pubkey, // Directly pass the value instead of a reference
            amm_account_bump,
        );
        let event_authority_key = Pubkey::new_from_array([
            28, 162, 68, 148, 13, 73, 10, 218, 71, 192, 253, 204, 79, 197, 229, 138, 223, 174, 109,
            177, 217, 166, 197, 97, 59, 9, 196, 88, 60, 84, 235, 243,
        ]);

        let (global_parameters, global_parameters_bump) = state.new_global_parameters(
            Rent::free(),
            &global_parameters_pubkey,
            global_parameters_bump,
        );
        PumpAmmFuzzContext {
            owner,
            system_program,
            token_program,
            state,
            base_mint,
            quote_mint,
            lp_mint,
            program_id,
            amm,
            amm_account_bump,
            event_authority: event_authority_key,
            global_parameters,
            global_parameters_bump,
        }
    }

    pub fn process_initialize<'a: 'bump>(
        &'a self,
        program_id: &'bump Pubkey,
        rent: Rent,
        global_parameters: &'bump AccountInfo<'bump>,
        global_parameters_bump: u8,

        event_authority: &'bump Pubkey,
    ) {
        let admin = Signer::try_from(&self.owner).unwrap();
        let event_authority = self.state.new_event_authority(&self.event_authority);
        // Creating a persistent binding for program_id to extend its lifetime
        let program_id_binding = pump_v2_amm::id();

        // Similarly, ensure Initialize structure lives long enough
        let mut init_struct = crate::state::Initialize {
            admin,
            global_parameters: Account::try_from(&global_parameters).unwrap(),
            system_program: Program::try_from(&self.system_program).unwrap(),
            // event_authority,
            // program: self.state.new_program(crate::id()),
        };

        // Creating a vector of global_parameters' pubkey to pass as a reference
        let gp_vec = vec![global_parameters.clone()];

        let context = Context::new(
            &program_id_binding,
            &mut init_struct,
            &gp_vec,
            pump_v2_amm::state::InitializeBumps {
                global_parameters: global_parameters_bump,
                //event_authority: *self.state.bump.alloc(0),
            },
        );

        pump_v2_amm::solana_amm::initialize(context).unwrap();
    }

    // Create system program account
    pub fn setup_system_program(state: &'bump AccountsState) -> AccountInfo<'bump> {
        state.new_program(system_program::id())
    }

    // Create token program account
    pub fn setup_token_program(state: &'bump AccountsState) -> AccountInfo<'bump> {
        state.new_program(spl_token::id())
    }

    // Setup owner account
    pub fn setup_owner_account(state: &'bump AccountsState) -> AccountInfo<'bump> {
        state.new_sol_account(1_000_000_000)
    }
}

#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use anchor_lang::prelude::*;
    use pump_v2_amm::instructions::set_parameters;
    use std::collections::BTreeMap;
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
            Pubkey::new_from_array([
                28, 162, 68, 148, 13, 73, 10, 218, 71, 192, 253, 204, 79, 197, 229, 138, 223, 174,
                109, 177, 217, 166, 197, 97, 59, 9, 196, 88, 60, 84, 235, 243,
            ]),
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
            Pubkey::new_from_array([
                28, 162, 68, 148, 13, 73, 10, 218, 71, 192, 253, 204, 79, 197, 229, 138, 223, 174,
                109, 177, 217, 166, 197, 97, 59, 9, 196, 88, 60, 84, 235, 243,
            ]),
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
            Pubkey::new_from_array([
                28, 162, 68, 148, 13, 73, 10, 218, 71, 192, 253, 204, 79, 197, 229, 138, 223, 174,
                109, 177, 217, 166, 197, 97, 59, 9, 196, 88, 60, 84, 235, 243,
            ]),
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
            Pubkey::new_from_array([
                28, 162, 68, 148, 13, 73, 10, 218, 71, 192, 253, 204, 79, 197, 229, 138, 223, 174,
                109, 177, 217, 166, 197, 97, 59, 9, 196, 88, 60, 84, 235, 243,
            ]),
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
            Pubkey::new_from_array([
                28, 162, 68, 148, 13, 73, 10, 218, 71, 192, 253, 204, 79, 197, 229, 138, 223, 174,
                109, 177, 217, 166, 197, 97, 59, 9, 196, 88, 60, 84, 235, 243,
            ]),
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
#[cfg(test)]
mod tests {
    use fixed::types::I80F48;
    use marginfi::state::marginfi_account::RiskEngine;
    use pyth_sdk_solana::state::PriceAccount;

    use super::*;
    #[test]
    pub fn deposit_test() {
        let account_state = AccountsState::new();

        let a = MarginfiFuzzContext::setup(&account_state, &[BankAndOracleConfig::dummy(); 2], 2);

        let al = AccountLoader::<MarginfiGroup>::try_from_unchecked(
            &pump_v2_amm::id(),
            &a.marginfi_group,
        )
        .unwrap();

        assert_eq!(al.load().unwrap().admin, a.owner.key());

        a.process_action_deposit(&AccountIdx(0), &BankIdx(0), &AssetAmount(1000))
            .unwrap();

        let marginfi_account_ai = AccountLoader::<MarginfiAccount>::try_from_unchecked(
            &pump_v2_amm::id(),
            &a.marginfi_accounts[0].margin_account,
        )
        .unwrap();
        let marginfi_account = marginfi_account_ai.load().unwrap();

        assert_eq!(
            I80F48::from(marginfi_account.lending_account.balances[0].asset_shares),
            I80F48!(1000)
        );
    }

    #[test]
    pub fn borrow_test() {
        let account_state = AccountsState::new();
        let a = MarginfiFuzzContext::setup(&account_state, &[BankAndOracleConfig::dummy(); 2], 2);

        a.process_action_deposit(&AccountIdx(1), &BankIdx(1), &AssetAmount(1000))
            .unwrap();
        a.process_action_deposit(&AccountIdx(0), &BankIdx(0), &AssetAmount(1000))
            .unwrap();
        a.process_action_borrow(&AccountIdx(0), &BankIdx(1), &AssetAmount(100))
            .unwrap();

        let marginfi_account_ai = AccountLoader::<MarginfiAccount>::try_from_unchecked(
            &pump_v2_amm::id(),
            &a.marginfi_accounts[0].margin_account,
        )
        .unwrap();

        {
            let marginfi_account = marginfi_account_ai.load().unwrap();

            assert_eq!(
                I80F48::from(marginfi_account.lending_account.balances[0].asset_shares),
                I80F48!(1000)
            );
            assert_eq!(
                I80F48::from(marginfi_account.lending_account.balances[1].liability_shares),
                I80F48!(100)
            );
        }

        a.process_action_repay(&AccountIdx(0), &BankIdx(1), &AssetAmount(100), false)
            .unwrap();

        let marginfi_account = marginfi_account_ai.load().unwrap();

        assert_eq!(
            I80F48::from(marginfi_account.lending_account.balances[1].liability_shares),
            I80F48!(0)
        );
    }

    #[test]
    pub fn liquidation_test() {
        let account_state = AccountsState::new();
        let a = MarginfiFuzzContext::setup(&account_state, &[BankAndOracleConfig::dummy(); 2], 3);

        a.process_action_deposit(&AccountIdx(1), &BankIdx(1), &AssetAmount(1000))
            .unwrap();
        a.process_action_deposit(&AccountIdx(0), &BankIdx(0), &AssetAmount(1000))
            .unwrap();
        a.process_action_borrow(&AccountIdx(0), &BankIdx(1), &AssetAmount(500))
            .unwrap();

        a.banks[1].log_oracle_price().unwrap();

        a.process_update_oracle(&BankIdx(1), &PriceChange(10000000000000))
            .unwrap();

        a.banks[1].log_oracle_price().unwrap();

        let marginfi_account_ai = AccountLoader::<MarginfiAccount>::try_from_unchecked(
            &pump_v2_amm::id(),
            &a.marginfi_accounts[0].margin_account,
        )
        .unwrap();

        {
            let marginfi_account = marginfi_account_ai.load().unwrap();
            let margin_account = &a.marginfi_accounts[0];
            let bank_map = a.get_bank_map();
            let remaining_accounts =
                &margin_account.get_remaining_accounts(&bank_map, vec![], vec![]);

            let re = RiskEngine::new(&marginfi_account, remaining_accounts).unwrap();

            let health = re
                .get_account_health(
                    marginfi::state::marginfi_account::RiskRequirementType::Maintenance,
                )
                .unwrap();

            println!("Health {health}");
        }

        a.process_action_deposit(&AccountIdx(2), &BankIdx(1), &AssetAmount(1000))
            .unwrap();

        a.process_liquidate_account(&AccountIdx(2), &AccountIdx(0), &AssetAmount(50))
            .unwrap();

        let marginfi_account_ai = AccountLoader::<MarginfiAccount>::try_from_unchecked(
            &pump_v2_amm::id(),
            &a.marginfi_accounts[0].margin_account,
        )
        .unwrap();

        let marginfi_account = marginfi_account_ai.load().unwrap();

        assert_eq!(
            I80F48::from(marginfi_account.lending_account.balances[0].asset_shares),
            I80F48!(950)
        );
    }

    #[test]
    pub fn liquidation_and_bankruptcy() {
        let account_state = AccountsState::new();

        let a = MarginfiFuzzContext::setup(&account_state, &[BankAndOracleConfig::dummy(); 2], 3);

        a.process_action_deposit(&AccountIdx(1), &BankIdx(1), &AssetAmount(1000))
            .unwrap();
        a.process_action_deposit(&AccountIdx(0), &BankIdx(0), &AssetAmount(1000))
            .unwrap();
        a.process_action_borrow(&AccountIdx(0), &BankIdx(1), &AssetAmount(500))
            .unwrap();

        a.process_update_oracle(&BankIdx(1), &PriceChange(1000000000000))
            .unwrap();

        let marginfi_account_ai = AccountLoader::<MarginfiAccount>::try_from_unchecked(
            &pump_v2_amm::id(),
            &a.marginfi_accounts[0].margin_account,
        )
        .unwrap();

        {
            let marginfi_account = marginfi_account_ai.load().unwrap();
            let margin_account = &a.marginfi_accounts[0];
            let bank_map = a.get_bank_map();
            let remaining_accounts =
                &margin_account.get_remaining_accounts(&bank_map, vec![], vec![]);

            let re = RiskEngine::new(&marginfi_account, remaining_accounts).unwrap();

            let health = re
                .get_account_health(
                    marginfi::state::marginfi_account::RiskRequirementType::Maintenance,
                )
                .unwrap();

            println!("Health {health}");
        }

        a.process_action_deposit(&AccountIdx(2), &BankIdx(1), &AssetAmount(1000))
            .unwrap();

        a.process_liquidate_account(&AccountIdx(2), &AccountIdx(0), &AssetAmount(1000))
            .unwrap();

        let marginfi_account_ai = AccountLoader::<MarginfiAccount>::try_from_unchecked(
            &pump_v2_amm::id(),
            &a.marginfi_accounts[0].margin_account,
        )
        .unwrap();

        let marginfi_account = marginfi_account_ai.load().unwrap();

        assert_eq!(
            I80F48::from(marginfi_account.lending_account.balances[0].asset_shares),
            I80F48!(0)
        );
        assert_eq!(
            I80F48::from(marginfi_account.lending_account.balances[0].liability_shares),
            I80F48!(0)
        );
    }

    #[test]
    pub fn price_update() {
        let account_state = AccountsState::new();

        let a = MarginfiFuzzContext::setup(&account_state, &[BankAndOracleConfig::dummy(); 2], 3);

        let price = {
            let data = a.banks[0].oracle.try_borrow_data().unwrap();
            let data = bytemuck::from_bytes::<PriceAccount>(&data);

            data.ema_price.val
        };

        a.process_update_oracle(&BankIdx(0), &PriceChange(1100))
            .unwrap();

        let new_price = {
            let data = a.banks[0].oracle.try_borrow_data().unwrap();
            let data = bytemuck::from_bytes::<PriceAccount>(&data);
            data.ema_price.val
        };

        assert_eq!(price, new_price - 1100);
    }
}
