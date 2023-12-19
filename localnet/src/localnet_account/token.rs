use solana_program::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use std::io::Write;
use std::ops::{Deref, DerefMut};
use spl_token::state::AccountState;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TokenAccount(spl_token::state::Account);

impl TokenAccount {
    pub const LEN: usize = spl_token::state::Account::LEN;

    pub fn new(
        mint: Pubkey,
        owner: Pubkey,
        amount: u64,
    ) -> Self {
        Self::from(spl_token::state::Account {
            mint,
            owner,
            amount,
            delegate: Default::default(),
            state: AccountState::Initialized,
            is_native: Default::default(),
            delegated_amount: 0,
            close_authority: Default::default(),
        })
    }

    pub fn mint(mut self, mint: Pubkey) -> Self {
        self.mint = mint;
        self
    }

    pub fn owner(mut self, owner: Pubkey) -> Self {
        self.owner = owner;
        self
    }

    pub fn amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }

    pub fn state(mut self, state: AccountState) -> Self {
        self.state = state;
        self
    }

    pub fn is_native(mut self, is_native: Option<u64>) -> Self {
        self.is_native = is_native.into();
        self
    }

    pub fn delegated(mut self, delegate: Option<Pubkey>, delegated_amount: u64) -> Self {
        self.delegate = delegate.into();
        self.delegated_amount = delegated_amount;
        self
    }

    pub fn close_authority(mut self, close_authority: Option<Pubkey>) -> Self {
        self.close_authority = close_authority.into();
        self
    }
}

impl anchor_lang::AccountDeserialize for TokenAccount {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        spl_token::state::Account::unpack(buf)
            .map(TokenAccount)
            .map_err(Into::into)
    }
}

impl anchor_lang::AccountSerialize for TokenAccount {
    fn try_serialize<W: Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
        let mut data = vec![0; spl_token::state::Account::get_packed_len()];
        spl_token::state::Account::pack(self.0, &mut data)?;
        writer
            .write(&data)
            .map_err(Into::<anchor_lang::error::Error>::into)?;
        Ok(())
    }
}

impl anchor_lang::Owner for TokenAccount {
    fn owner() -> Pubkey {
        spl_token::ID
    }
}

impl Deref for TokenAccount {
    type Target = spl_token::state::Account;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TokenAccount {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<spl_token::state::Account> for TokenAccount {
    fn from(value: spl_token::state::Account) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mint(spl_token::state::Mint);

impl Mint {
    pub const LEN: usize = spl_token::state::Mint::LEN;

    pub fn new(
        mint_authority: Option<Pubkey>,
        supply: u64,
        decimals: u8,
    ) -> Self {
        Self::from(spl_token::state::Mint {
            mint_authority: mint_authority.into(),
            supply,
            decimals,
            is_initialized: true,
            freeze_authority: Default::default(),
        })
    }

    pub fn mint_authority(mut self, mint_authority: Option<Pubkey>) -> Self {
        self.mint_authority = mint_authority.into();
        self
    }

    pub fn supply(mut self, supply: u64) -> Self {
        self.supply = supply;
        self
    }

    pub fn decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }

    pub fn is_initialized(mut self, is_initialized: bool) -> Self {
        self.is_initialized = is_initialized;
        self
    }

    pub fn freeze_authority(mut self, freeze_authority: Option<Pubkey>) -> Self {
        self.freeze_authority = freeze_authority.into();
        self
    }
}

impl anchor_lang::AccountDeserialize for Mint {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        spl_token::state::Mint::unpack(buf)
            .map(Mint)
            .map_err(Into::into)
    }
}

impl anchor_lang::AccountSerialize for Mint {
    fn try_serialize<W: Write>(&self, writer: &mut W) -> anchor_lang::Result<()> {
        let mut data = vec![0; spl_token::state::Mint::get_packed_len()];
        spl_token::state::Mint::pack(self.0, &mut data)?;
        writer
            .write(&data)
            .map_err(Into::<anchor_lang::error::Error>::into)?;
        Ok(())
    }
}

impl anchor_lang::Owner for Mint {
    fn owner() -> Pubkey {
        spl_token::ID
    }
}

impl Deref for Mint {
    type Target = spl_token::state::Mint;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Mint {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<spl_token::state::Mint> for Mint {
    fn from(value: spl_token::state::Mint) -> Self {
        Self(value)
    }
}
