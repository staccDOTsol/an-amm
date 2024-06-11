#![no_main]

use anchor_lang::prelude::*;
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use pump_amm_fuzz::{AccountsState, PumpAmmFuzzContext};
use std::sync::{Arc, RwLock};
#[derive(Debug, Clone)]
pub struct WrappedPubkey(Pubkey);
impl From<&WrappedPubkey> for Pubkey {
    fn from(wrapped: &WrappedPubkey) -> Self {
        wrapped.0
    }
}

impl<'a> Arbitrary<'a> for WrappedPubkey {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(WrappedPubkey(Pubkey::new_unique()))
    }
}
#[derive(Debug, Arbitrary)]
enum Action {
    Create {
        account: WrappedPubkey,
        amm: WrappedPubkey,
        program_id: WrappedPubkey,
        bump_seed: WrappedPubkey,
        event_authority: WrappedPubkey,
    },
    AddLiquidity {
        amm_pubkey: WrappedPubkey,
        base_pubkey: WrappedPubkey,
        quote_pubkey: WrappedPubkey,
        payer_pubkey: WrappedPubkey,
        lp_mint_pubkey: WrappedPubkey,
        base_amount: u64,
        quote_amount: u64,
        min_liquidity: u64,
    },
    RemoveLiquidity {
        amm_pubkey: WrappedPubkey,
        base_pubkey: WrappedPubkey,
        quote_pubkey: WrappedPubkey,
        payer_pubkey: WrappedPubkey,
        lp_mint_pubkey: WrappedPubkey,
        shares: u64,
        quote_min_amount: u64,
        base_min_amount: u64,
    },
    Buy {
        amm_pubkey: WrappedPubkey,
        global_parameters: WrappedPubkey,
        base_pubkey: WrappedPubkey,
        quote_pubkey: WrappedPubkey,
        payer_pubkey: WrappedPubkey,
        base_amount: u64,
        max_quote_amount: u64,
    },
    Sell {
        amm_pubkey: WrappedPubkey,
        global_parameters: WrappedPubkey,
        base_pubkey: WrappedPubkey,
        quote_pubkey: WrappedPubkey,
        payer_pubkey: WrappedPubkey,
        base_amount: u64,
        min_quote_amount: u64,
    },
    Initialize {
        payer_pubkey: WrappedPubkey,
        global_parameters: WrappedPubkey,
        program_id: WrappedPubkey,
        bump_seed: WrappedPubkey,
        event_authority: WrappedPubkey,
    },
    SetParameters {
        amm_pubkey: WrappedPubkey,
        new_parameters: WrappedPubkey,
    },
}

#[derive(Debug)]
pub struct ActionSequence(Vec<Action>);

impl<'a> Arbitrary<'a> for ActionSequence {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let n_actions = 100;
        let mut actions = Vec::with_capacity(n_actions);

        for _ in 0..n_actions {
            let action = Action::arbitrary(u)?;
            actions.push(action);
        }

        Ok(ActionSequence(actions))
    }
}

#[derive(Debug, Arbitrary)]
pub struct FuzzerContext {
    pub action_sequence: ActionSequence,
}

fuzz_target!(|data: FuzzerContext| process_actions(data).unwrap());

// Setup base and quote mints
pub fn setup_mints(state: &AccountsState) -> (AccountInfo, AccountInfo, AccountInfo) {
    let base_mint = state.new_token_mint(Rent::free(), 9);
    let quote_mint = state.new_token_mint(Rent::free(), 9);
    let lp_mint = state.new_token_mint(Rent::free(), 9);
    (base_mint, quote_mint, lp_mint)
}

fn process_actions(ctx: FuzzerContext) -> Result<()> {
    let mut accounts_state = AccountsState::new();
    let (base_mint, quote_mint, lp_mint) = setup_mints(&accounts_state);
    let program_id = PumpAmmFuzzContext::get_program_id();
    let owner = accounts_state.new_sol_account(1_000_000_000);
    let (amm_account_pubkey, amm_account_bump) = Pubkey::find_program_address(
        &[
            b"amm",
            owner.key.as_ref(),
            base_mint.key.as_ref(),
            quote_mint.key.as_ref(),
        ],
        &PumpAmmFuzzContext::get_program_id(),
    );
    let (global_parameters_pubkey, global_parameters_bump) =
        PumpAmmFuzzContext::get_global_parameters(&PumpAmmFuzzContext::get_program_id());
    let mut context = PumpAmmFuzzContext::setup(
        &accounts_state,
        &lp_mint,
        &base_mint,
        &quote_mint,
        &amm_account_pubkey,
        amm_account_bump,
        &global_parameters_pubkey,
        global_parameters_bump,
        &owner,
    );

    for action in ctx.action_sequence.0.iter() {
        process_action(action, &context)?;
    }

    accounts_state.reset();

    Ok(())
}
fn process_action<'bump>(action: &Action, context: &'bump PumpAmmFuzzContext<'bump>) -> Result<()> {
    match action {
        Action::Initialize {
            payer_pubkey,
            global_parameters,
            program_id,
            bump_seed,
            event_authority,
        } => context.process_initialize(
            &context.program_id,
            Rent::free(),
            &context.global_parameters,
            context.global_parameters_bump,
            &context.event_authority,
        ),
        Action::Create {
            account,
            amm,
            program_id,
            bump_seed,
            event_authority,
        } => {
            PumpAmmFuzzContext::setup_amm_account(
                context,
                &context.owner,
                &context.base_mint,
                &context.quote_mint,
                &context.lp_mint,
                context.amm_account_bump,
                &context.amm,
            );
        }
        _ => todo!(), /* handle other actions */
                      /*
                                            Action::AddLiquidity {
                                                amm_pubkey,
                                                base_pubkey,
                                                quote_pubkey,
                                                payer_pubkey,
                                                lp_mint_pubkey,
                                                base_amount,
                                                quote_amount,
                                                min_liquidity,
                                            } => {
                                                // Implement the logic for adding liquidity
                                            }
                                            Action::RemoveLiquidity {
                                                amm_pubkey,
                                                base_pubkey,
                                                quote_pubkey,
                                                payer_pubkey,
                                                lp_mint_pubkey,
                                                shares,
                                                quote_min_amount,
                                                base_min_amount,
                                            } => {
                                                // Implement the logic for removing liquidity
                                            }
                                            Action::Buy {
                                                amm_pubkey,
                                                global_parameters,
                                                base_pubkey,
                                                quote_pubkey,
                                                payer_pubkey,
                                                base_amount,
                                                max_quote_amount,
                                            } => {
                                                // Implement the logic for buying
                                            }
                                            Action::Sell {
                                                amm_pubkey,
                                                global_parameters,
                                                base_pubkey,
                                                quote_pubkey,
                                                payer_pubkey,
                                                base_amount,
                                                min_quote_amount,
                                            } => {
                                                // Implement the logic for selling

                                            Action::SetParameters {
                                                amm_pubkey,
                                                new_parameters,
                                            } => {
                                                // Implement the logic for setting parameters
                                            }*/
    }
    Ok(())
}
