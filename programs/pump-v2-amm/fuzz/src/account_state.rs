use crate::SplAccount;
use anchor_lang::{
    prelude::{AccountInfo, Pubkey, Rent, SolanaSysvar},
    Discriminator,
};
use bumpalo::Bump;
use pump_v2_amm::state::Amm;
use safe_transmute::{transmute_to_bytes, transmute_to_bytes_mut};
use solana_program::{
    bpf_loader, program_pack::Pack, stake_history::Epoch, system_program, sysvar,
};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token::state::Mint;
use std::mem::size_of;

pub struct AccountsState {
    pub bump: Bump,
}

impl AccountsState {
    pub fn new() -> Self {
        Self { bump: Bump::new() }
    }

    fn random_pubkey<'bump>(&'bump self) -> &Pubkey {
        self.bump
            .alloc(Pubkey::new(transmute_to_bytes(&rand::random::<[u64; 4]>())))
    }

    pub fn new_sol_account<'bump>(&'bump self, lamports: u64) -> AccountInfo<'bump> {
        self.new_sol_account_with_pubkey(self.random_pubkey(), lamports)
    }

    pub fn new_sol_account_with_pubkey<'bump>(
        &'bump self,
        pubkey: &'bump Pubkey,
        lamports: u64,
    ) -> AccountInfo<'bump> {
        AccountInfo::new(
            pubkey,
            true,
            false,
            self.bump.alloc(lamports),
            &mut [],
            &system_program::ID,
            false,
            Epoch::default(),
        )
    }
    pub fn new_lp_mint<'bump>(&'bump self, rent: Rent, decimals: u8) -> AccountInfo {
        let data = self.bump.alloc_slice_fill_copy(Mint::LEN, 0u8);
        let mut mint = Mint::default();
        mint.is_initialized = false;
        mint.decimals = decimals;
        Mint::pack(mint, data).unwrap();
        AccountInfo::new(
            self.random_pubkey(),
            false, // is_signer
            false, // is_writable
            self.bump.alloc(rent.minimum_balance(Mint::LEN)),
            data,                                        // data is empty
            &spl_token::ID,                              // owner is set to default
            false,                                       // executable flag
            solana_sdk::stake_history::Epoch::default(), // epoch
        )
    }
    pub fn new_token_mint<'bump>(&'bump self, rent: Rent, decimals: u8) -> AccountInfo<'bump> {
        let data = self.bump.alloc_slice_fill_copy(Mint::LEN, 0u8);
        let mut mint = Mint::default();
        mint.is_initialized = true;
        mint.decimals = decimals;
        Mint::pack(mint, data).unwrap();
        AccountInfo::new(
            self.random_pubkey(),
            false,
            true,
            self.bump.alloc(rent.minimum_balance(data.len())),
            data,
            &spl_token::ID,
            false,
            Epoch::default(),
        )
    }

    pub fn new_token_account<'bump, 'a, 'b>(
        &'bump self,
        mint_pubkey: &'a Pubkey,
        owner_pubkey: &'b Pubkey,
        balance: u64,
        rent: Rent,
    ) -> AccountInfo<'bump> {
        self.new_token_account_with_pubkey(
            Keypair::new().pubkey(),
            mint_pubkey,
            owner_pubkey,
            balance,
            rent,
        )
    }

    pub fn new_token_account_with_pubkey<'bump, 'a, 'b>(
        &'bump self,
        account_pubkey: Pubkey,
        mint_pubkey: &'a Pubkey,
        owner_pubkey: &'b Pubkey,
        balance: u64,
        rent: Rent,
    ) -> AccountInfo<'bump> {
        let data = self.bump.alloc_slice_fill_copy(SplAccount::LEN, 0u8);
        let mut account = SplAccount::default();
        account.state = spl_token::state::AccountState::Initialized;
        account.mint = *mint_pubkey;
        account.owner = *owner_pubkey;
        account.amount = balance;
        SplAccount::pack(account, data).unwrap();
        AccountInfo::new(
            self.bump.alloc(account_pubkey),
            false,
            true,
            self.bump.alloc(rent.minimum_balance(data.len())),
            data,
            &spl_token::ID,
            false,
            Epoch::default(),
        )
    }

    pub fn new_owned_account<'bump>(
        &'bump self,
        unpadded_len: usize,
        owner_pubkey: Pubkey,
        rent: Rent,
    ) -> AccountInfo<'bump> {
        let data_len = unpadded_len + 12;
        self.new_dex_owned_account_with_lamports(
            unpadded_len,
            rent.minimum_balance(data_len),
            self.bump.alloc(owner_pubkey),
        )
    }
    pub fn new_amm_account<'bump>(
        &'bump self,
        owner_pubkey: Pubkey,
        rent: Rent,
        amm_account: &'bump Pubkey,
        amm_account_bump: u8,
    ) -> (AccountInfo<'bump>, u8) {
        let amm = (
            AccountInfo::new(
                &amm_account,
                false,
                true,
                self.bump
                    .alloc(rent.minimum_balance(std::mem::size_of::<crate::state::Amm>())),
                self.allocate_dex_owned_account(std::mem::size_of::<crate::state::Amm>()),
                &crate::ID,
                false,
                Epoch::default(),
            ),
            amm_account_bump,
        );
        amm
    }
    pub fn new_global_parameters<'bump>(
        &'bump self,
        rent: Rent,
        global_parameters: &'bump solana_sdk::pubkey::Pubkey,
        global_parameters_bump: u8,
    ) -> (AccountInfo<'bump>, u8) {
        (
            AccountInfo::new(
                &global_parameters,
                false,
                true,
                self.bump.alloc(
                    rent.minimum_balance(std::mem::size_of::<crate::state::GlobalParameters>()),
                ),
                self.allocate_dex_owned_account(
                    std::mem::size_of::<crate::state::GlobalParameters>(),
                ),
                &crate::ID,
                false,
                Epoch::default(),
            ),
            global_parameters_bump,
        )
    }
    pub fn new_dex_owned_account_with_lamports<'bump>(
        &'bump self,
        unpadded_len: usize,
        lamports: u64,
        program_id: &'bump Pubkey,
    ) -> AccountInfo<'bump> {
        AccountInfo::new(
            self.random_pubkey(),
            false,
            true,
            self.bump.alloc(lamports),
            self.allocate_dex_owned_account(unpadded_len),
            program_id,
            false,
            Epoch::default(),
        )
    }

    pub fn allocate_dex_owned_account<'bump>(&'bump self, unpadded_size: usize) -> &mut [u8] {
        assert_eq!(unpadded_size % 8, 0);
        let padded_size = unpadded_size + 12;
        let u64_data = self.bump.alloc_slice_fill_copy(padded_size / 8 + 1, 0u64);

        transmute_to_bytes_mut(u64_data) as _
    }

    pub fn new_spl_token_program(&self) -> AccountInfo {
        self.new_program(spl_token::id())
    }

    pub fn new_system_program(&self) -> AccountInfo {
        self.new_program(system_program::id())
    }

    pub fn new_marginfi_program(&self) -> AccountInfo {
        self.new_program(pump_v2_amm::id())
    }
    pub fn new_event_authority<'a>(&'a self, pubkey: &'a Pubkey) -> AccountInfo {
        AccountInfo::new(
            pubkey,
            false,
            true,
            self.bump.alloc(0),
            self.allocate_dex_owned_account(0),
            &crate::ID,
            false,
            solana_sdk::stake_history::Epoch::default(),
        )
    }

    pub fn new_program(&self, pubkey: Pubkey) -> AccountInfo {
        AccountInfo::new(
            self.bump.alloc(pubkey),
            false,
            false,
            self.bump.alloc(0),
            &mut [],
            &bpf_loader::ID,
            true,
            Epoch::default(),
        )
    }

    pub fn new_rent_sysvar_account(&self, rent: Rent) -> AccountInfo {
        let data = self.bump.alloc_slice_fill_copy(size_of::<Rent>(), 0u8);
        let lamports = rent.minimum_balance(data.len());

        let mut account_info = AccountInfo::new(
            &sysvar::rent::ID,
            false,
            false,
            self.bump.alloc(lamports),
            data,
            &sysvar::ID,
            false,
            Epoch::default(),
        );

        rent.to_account_info(&mut account_info).unwrap();

        account_info
    }

    pub fn reset(&mut self) {
        self.bump.reset();
    }
}

pub struct AccountInfoCache<'bump> {
    account_data: Vec<Vec<u8>>,
    account_info: Vec<AccountInfo<'bump>>,
}

impl<'info> AccountInfoCache<'info> {
    pub fn new(ais: &[AccountInfo<'info>]) -> Self {
        let account_data = ais.iter().map(|ai| ai.data.borrow().to_owned()).collect();
        Self {
            account_data,
            account_info: ais.to_vec(),
        }
    }

    pub fn revert(&self) {
        for (ai, data) in self.account_info.iter().zip(self.account_data.iter()) {
            ai.data.borrow_mut().copy_from_slice(data);
        }
    }
}
pub fn set_discriminator<T: Discriminator>(ai: AccountInfo) {
    let mut data = ai.try_borrow_mut_data().unwrap();

    if data[..8].ne(&[0u8; 8]) {
        panic!("Account discriminator is already set");
    }

    data[..8].copy_from_slice(&T::DISCRIMINATOR);
}
