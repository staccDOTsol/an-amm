use crate::error::ErrorCode;
use anchor_lang::{
    prelude::*,
    solana_program::{
        entrypoint::ProgramResult,
        instruction::Instruction,
        program::{invoke, invoke_signed},
    },
};
pub struct TokenTransferParams<'a: 'b, 'b> {
    pub source: AccountInfo<'a>,
    pub destination: AccountInfo<'a>,
    pub authority: AccountInfo<'a>,
    pub token_program: AccountInfo<'a>,
    pub amount: u64,
    pub decimals: u8,
    pub authority_signer_seeds: &'b [&'b [u8]],
    pub mint: AccountInfo<'a>,
}

/// Invoke signed unless signers seeds are empty
#[inline(always)]
fn invoke_optionally_signed(
    instruction: &Instruction,
    account_infos: &[AccountInfo],
    authority_signer_seeds: &[&[u8]],
) -> ProgramResult {
    if authority_signer_seeds.is_empty() {
        invoke(instruction, account_infos)
    } else {
        invoke_signed(instruction, account_infos, &[authority_signer_seeds])
    }
}

/// Issue a spl_token `Transfer` instruction.
#[inline(always)]
pub fn spl_token_transfer(params: TokenTransferParams<'_, '_>) -> Result<()> {
    let TokenTransferParams {
        source,
        destination,
        authority,
        token_program,
        amount,
        decimals,
        mint,
        authority_signer_seeds,
    } = params;
    let result;
    if token_program.key() == spl_token::ID {
        result = invoke_optionally_signed(
            &spl_token::instruction::transfer(
                token_program.key,
                source.key,
                destination.key,
                authority.key,
                &[],
                amount,
            )?,
            &[source, destination, authority, token_program],
            authority_signer_seeds,
        );
    } else {
        result = invoke_optionally_signed(
            &spl_token_2022::instruction::transfer_checked(
                token_program.key,
                source.key,
                mint.key,
                destination.key,
                authority.key,
                &[],
                amount,
                decimals,
            )?,
            &[source, destination, authority, token_program, mint],
            authority_signer_seeds,
        );
    }
    Ok(result.map_err(|_| ErrorCode::TokenTransferFailed)?)
}
