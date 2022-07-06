use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, SetAuthority, TokenAccount, Transfer};
use spl_token::instruction::AuthorityType;

declare_id!("HGhUfApRyEBL758VLG5kq45UkEAsvaVcPvCxVHuXMdhU");

#[program]
pub mod anchor_auction {
    use std::ops::Add;
    use super::*;

    const ESCROW_PDA_SEED: &[u8] = b"escrow";

    pub fn exhibit(
        ctx: Context<Exhibit>,
        initial_price: u64,
        auction_duration_sec: u64,
    ) -> Result<()> {
        ctx.accounts.escrow_account.exhibitor_pubkey = ctx.accounts.exhibitor.key();
        ctx.accounts.escrow_account.exhibitor_ft_receiving_pubkey = ctx.accounts.exhibitor_ft_receiving_account.key();
        ctx.accounts.escrow_account.exhibiting_nft_temp_pubkey = ctx.accounts.exhibitor_nft_temp_account.key();
        ctx.accounts.escrow_account.highest_bidder_pubkey = ctx.accounts.exhibitor.key();
        ctx.accounts.escrow_account.highest_bidder_ft_temp_pubkey = ctx.accounts.exhibitor_ft_receiving_account.key();
        ctx.accounts.escrow_account.highest_bidder_ft_returning_pubkey = ctx.accounts.exhibitor_ft_receiving_account.key();
        ctx.accounts.escrow_account.price = initial_price;
        ctx.accounts.escrow_account.end_at = ctx.accounts.clock.unix_timestamp.add(auction_duration_sec as i64);

        let (pda, _bump_seed) = Pubkey::find_program_address(&[ESCROW_PDA_SEED], ctx.program_id);
        token::set_authority(
            ctx.accounts.to_set_authority_context(),
        AuthorityType::AccountOwner,
        Some(pda)
        )?;

        token::transfer(
            ctx.accounts.to_transfer_to_pda_context(),
           1
        )?;

        Ok(())
    }

    pub fn cancel(ctx: Context<Cancel> ) -> Result<()> {
        let (_, bump_seed) = Pubkey::find_program_address(&[ESCROW_PDA_SEED], ctx.program_id);
        let signers_seeds: &[&[&[u8]]] = &[&[&ESCROW_PDA_SEED[..], &[bump_seed]]];

        token::transfer(
            ctx.accounts
                .to_transfer_to_exhibitor_context()
                .with_signer(signers_seeds),
            ctx.accounts.exhibitor_nft_temp_account.amount
        )?;

        token::close_account(
            ctx.accounts
                .to_close_context()
                .with_signer(signers_seeds)
        )?;

        Ok(())
    }

    pub fn bid(ctx: Context<Bid>, price: u64) -> Result<()> {
        let (pda, bump_seed) = Pubkey::find_program_address(&[ESCROW_PDA_SEED], ctx.program_id);
        let signers_seeds: &[&[&[u8]]] = &[&[&ESCROW_PDA_SEED[..], &[bump_seed]]];

        if ctx.accounts.escrow_account.highest_bidder_pubkey != ctx.accounts.escrow_account.exhibitor_pubkey {
            token::transfer(
                ctx.accounts
                    .to_transfer_to_previous_bidder_context()
                    .with_signer(signers_seeds),
                ctx.accounts.escrow_account.price
            )?;

            token::close_account(
                ctx.accounts
                    .to_close_context()
                    .with_signer(signers_seeds)
            )?;
        }

        token::set_authority(
            ctx.accounts.to_set_authority_context(),
            AuthorityType::AccountOwner,
            Some(pda)
        )?;
        token::transfer(
            ctx.accounts.to_transfer_to_pda_context(),
            price,
        )?;

        ctx.accounts.escrow_account.price = price;
        ctx.accounts.escrow_account.highest_bidder_pubkey = ctx.accounts.bidder.key();
        ctx.accounts.escrow_account.highest_bidder_ft_temp_pubkey = ctx.accounts.bidder_ft_temp_account.key();
        ctx.accounts.escrow_account.highest_bidder_ft_returning_pubkey = ctx.accounts.bidder_ft_account.key();

        Ok(())
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        let (_, bump_seed) = Pubkey::find_program_address(&[ESCROW_PDA_SEED], ctx.program_id);
        let signers_seeds: &[&[&[u8]]] = &[&[&ESCROW_PDA_SEED[..], &[bump_seed]]];

        token::transfer(
            ctx.accounts
                .to_transfer_to_highest_bidder_context()
                .with_signer(signers_seeds),
            ctx.accounts.exhibitor_nft_temp_account.amount,
        )?;

        token::transfer(
            ctx.accounts
                .to_transfer_to_exhibitor_context()
                .with_signer(signers_seeds),
            ctx.accounts.highest_bidder_ft_temp_account.amount,
        )?;

        token::close_account(
            ctx.accounts.to_close_ft_context()
                .with_signer(signers_seeds),
        )?;

        token::close_account(
            ctx.accounts.to_close_nft_context()
                .with_signer(signers_seeds),
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(initial_price: u64, auction_duration_sec: u64)]
pub struct Exhibit<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(signer)]
    pub exhibitor: AccountInfo<'info>,
    #[account(
        mut,
        constraint = exhibitor_nft_token_account.amount == 1
    )]
    pub exhibitor_nft_token_account: Account<'info, TokenAccount>,
    pub exhibitor_nft_temp_account: Account<'info, TokenAccount>,
    pub exhibitor_ft_receiving_account:Account<'info, TokenAccount>,
    #[account(zero)]
    pub escrow_account: Box<Account<'info, Auction>>,
    pub clock: Sysvar<'info, Clock>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(signer)]
    pub exhibitor: AccountInfo<'info>,
    #[account(mut)]
    pub exhibitor_nft_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub exhibitor_nft_temp_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = escrow_account.exhibitor_pubkey == exhibitor.key(),
        constraint = escrow_account.highest_bidder_pubkey == exhibitor.key(),
        constraint = escrow_account.exhibiting_nft_temp_pubkey == exhibitor_nft_temp_account.key(),
        close = exhibitor
    )]
    pub escrow_account: Box<Account<'info, Auction>>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub pda: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(price: u64)]
pub struct Bid<'info> {
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(signer)]
    pub bidder: AccountInfo<'info>,
    #[account(mut)]
    pub bidder_ft_temp_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = bidder_ft_account.amount >= price
    )]
    pub bidder_ft_account: Account<'info, TokenAccount>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    #[account(
        mut,
        constraint = highest_bidder.key() != bidder.key()
    )]
    pub highest_bidder: AccountInfo<'info>,
    #[account(mut)]
    pub highest_bidder_ft_temp_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub highest_bidder_ft_returning_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = escrow_account.highest_bidder_pubkey == highest_bidder.key(),
        constraint = escrow_account.highest_bidder_ft_temp_pubkey == highest_bidder_ft_temp_account.key(),
        constraint = escrow_account.highest_bidder_ft_returning_pubkey == highest_bidder_ft_returning_account.key(),
        constraint = escrow_account.price < price,
        constraint = escrow_account.end_at > clock.unix_timestamp
    )]
    pub escrow_account: Box<Account<'info, Auction>>,
    pub clock: Sysvar<'info, Clock>,
    /// CHECK: This is not dangerous because we don't read or write from this account TODO check pda key
    pub pda: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(signer)]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub winning_bidder: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub exhibitor: AccountInfo<'info>,
    #[account(mut)]
    pub exhibitor_nft_temp_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub exhibitor_ft_receiving_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub highest_bidder_ft_temp_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub highest_bidder_nft_receiving_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = escrow_account.exhibitor_pubkey == exhibitor.key(),
        constraint = escrow_account.exhibiting_nft_temp_pubkey == exhibitor_nft_temp_account.key(),
        constraint = escrow_account.exhibitor_ft_receiving_pubkey == exhibitor_ft_receiving_account.key(),
        constraint = escrow_account.highest_bidder_pubkey == winning_bidder.key(),
        constraint = escrow_account.highest_bidder_ft_temp_pubkey == highest_bidder_ft_temp_account.key(),
        constraint = escrow_account.end_at <= clock.unix_timestamp,
        close = exhibitor
    )]
    pub escrow_account: Box<Account<'info, Auction>>,
    pub clock: Sysvar<'info, Clock>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub pda: AccountInfo<'info>,
    /// CHECK: This is not dangerous because we don't read or write from this account
    pub token_program: AccountInfo<'info>,
}

impl<'info> Exhibit<'info> {
    fn to_transfer_to_pda_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self
                .exhibitor_nft_token_account
                .to_account_info()
                .clone(),
            to: self.exhibitor_nft_temp_account.to_account_info().clone(),
            authority: self.exhibitor.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn to_set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.exhibitor_nft_temp_account.to_account_info().clone(),
            current_authority: self.exhibitor.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}


impl<'info> Cancel<'info> {
    fn to_transfer_to_exhibitor_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.exhibitor_nft_temp_account.to_account_info().clone(),
            to: self
                .exhibitor_nft_token_account
                .to_account_info()
                .clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn to_close_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.exhibitor_nft_temp_account.to_account_info().clone(),
            destination: self.exhibitor.clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

impl<'info> Bid<'info> {

    fn to_set_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.bidder_ft_temp_account.to_account_info().clone(),
            current_authority: self.bidder.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn to_close_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.highest_bidder_ft_temp_account.to_account_info().clone(),
            destination: self.highest_bidder.clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn to_transfer_to_previous_bidder_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.highest_bidder_ft_temp_account.to_account_info().clone(),
            to: self
                .highest_bidder_ft_returning_account
                .to_account_info()
                .clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn to_transfer_to_pda_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.bidder_ft_account.to_account_info().clone(),
            to: self
                .bidder_ft_temp_account
                .to_account_info()
                .clone(),
            authority: self.bidder.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

impl<'info> Close<'info> {
    fn to_transfer_to_exhibitor_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.highest_bidder_ft_temp_account.to_account_info().clone(),
            to: self
                .exhibitor_ft_receiving_account
                .to_account_info()
                .clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn to_transfer_to_highest_bidder_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.exhibitor_nft_temp_account.to_account_info().clone(),
            to: self
                .highest_bidder_nft_receiving_account
                .to_account_info()
                .clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn to_close_ft_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.highest_bidder_ft_temp_account.to_account_info().clone(),
            destination: self.winning_bidder.clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }

    fn to_close_nft_context(&self) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let cpi_accounts = CloseAccount {
            account: self.exhibitor_nft_temp_account.to_account_info().clone(),
            destination: self.exhibitor.clone(),
            authority: self.pda.clone(),
        };
        CpiContext::new(self.token_program.clone(), cpi_accounts)
    }
}

/// see https://github.com/yoshidan/solana-auction/blob/main/program/src/state.rs#L10
#[account]
pub struct Auction {
    pub exhibitor_pubkey: Pubkey,
    pub exhibiting_nft_temp_pubkey: Pubkey,
    pub exhibitor_ft_receiving_pubkey: Pubkey,
    pub price: u64,
    pub end_at: i64,
    pub highest_bidder_pubkey: Pubkey,
    pub highest_bidder_ft_temp_pubkey: Pubkey,
    pub highest_bidder_ft_returning_pubkey: Pubkey,
}
