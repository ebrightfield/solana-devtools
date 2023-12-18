use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod test_program {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        let clock = Clock::get()?;
        msg!("{:?}", clock);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    mint: Account<'info, Mint>,
    #[account(
        init,
        payer = owner,
        associated_token::mint = mint,
        associated_token::authority = owner,
    )]
    new_account: Account<'info, TokenAccount>,
    #[account(mut)]
    owner: Signer<'info>,
    token_program: Program<'info, Token>,
    /// Needed to create an associated token account
    associated_token_program: Program<'info, AssociatedToken>,
    /// Needed to create a new account
    system_program: Program<'info, System>,
    /// Needed to create an associated token account
    rent: Sysvar<'info, Rent>,
}
