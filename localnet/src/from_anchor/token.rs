use std::ops::Deref;
use std::io::Write;
use solana_sdk::pubkey::Pubkey;
use solana_program::program_pack::Pack;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TokenAccount(spl_token::state::Account);

impl TokenAccount {
    pub const LEN: usize = spl_token::state::Account::LEN;
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
        spl_token::state::Account::pack(
            self.0, &mut data,
        )?;
        writer.write(&data).map_err(Into::<anchor_lang::error::Error>::into)?;
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

impl From<spl_token::state::Account> for TokenAccount {
    fn from(value: spl_token::state::Account) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mint(spl_token::state::Mint);

impl Mint {
    pub const LEN: usize = spl_token::state::Mint::LEN;
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
        spl_token::state::Mint::pack(
            self.0, &mut data,
        )?;
        writer.write(&data).map_err(Into::<anchor_lang::error::Error>::into)?;
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

impl From<spl_token::state::Mint> for Mint {
    fn from(value: spl_token::state::Mint) -> Self {
        Self(value)
    }
}
