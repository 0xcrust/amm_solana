use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, MintTo, Transfer};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub const MAX_POOLS: usize = 10;

#[program]
pub mod torrent {
    use super::*;

    pub fn initialize(ctx: Context<InitializeTorrent>, _lt_decimals: u8) -> Result<()> {
        let torrent = &mut ctx.accounts.torrent;
        torrent.authority = ctx.accounts.authority.key();
        torrent.lt_mint = ctx.accounts.liquidity_token.key();
        torrent.total_lt_supply = 0;
        torrent.pools = [Pubkey::default(); MAX_POOLS];

        Ok(())
    }

    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        initial_x: u64,
        initial_y: u64,
    ) -> Result<()> {
        let state = &mut ctx.accounts.pool_state;
        let torrent = &mut ctx.accounts.torrent;

        state.torrent = torrent.key();
        state.lt_minted = 0;

        let mint_amount = (initial_x + initial_y) >> 1;

        torrent.total_lt_supply = torrent.total_lt_supply.checked_add(mint_amount).unwrap();
        state.lt_minted = state.lt_minted.checked_add(mint_amount).unwrap();

        let torrent_bump = *ctx.bumps.get("torrent").unwrap();
        let authority = ctx.accounts.authority.key();
        let torrent_signature = &[
            b"torrent", 
            authority.as_ref(), 
            &[torrent_bump]
        ];

        anchor_spl::token::mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    to: ctx.accounts.authority_lt_vault.to_account_info(),
                    mint: ctx.accounts.lt_mint.to_account_info(),
                    authority: ctx.accounts.torrent.to_account_info()
                }
            ).with_signer(&[&torrent_signature[..]]),
            mint_amount
        )?;

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.authority_x_wallet.to_account_info(),
                    to: ctx.accounts.x_token_vault.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info()
                }
            ),
            initial_x,
        )?;

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.authority_y_wallet.to_account_info(),
                    to: ctx.accounts.y_token_vault.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info()
                }
            ),
            initial_y,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(lt_decimals: u8)]
pub struct InitializeTorrent<'info> {
    #[account(mut)]
    authority: Signer<'info>,

    /// Stores torrent state
    #[account(
        init,
        seeds = [b"torrent".as_ref(), authority.key().as_ref()],
        bump,
        payer = authority,
        space = Torrent::SIZE
    )]
    torrent: Account<'info, Torrent>,

    /// This is the liquidity token for torrent's pools
    #[account(
        init,
        seeds = [b"token".as_ref(), torrent.key().as_ref()],
        bump,
        payer = authority,
        mint::decimals = lt_decimals,
        mint::authority = torrent,
    )]
    liquidity_token: Account<'info, Mint>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(initial_x: u64, initial_y: u64)]
pub struct InitializePool<'info> {
    #[account(has_one = authority, has_one = lt_mint)]
    torrent: Account<'info, Torrent>,
    lt_mint: Account<'info, Mint>,

    #[account(mut)]
    authority: Signer<'info>,

    /// Token pair for this pool
    mint_x: Account<'info, Mint>,
    mint_y: Account<'info, Mint>,

    /// Authority's token accounts
    #[account(
        mut,
        constraint = authority_x_wallet.owner == authority.key(),
        constraint = authority_x_wallet.mint == mint_x.key(),
        constraint = authority_x_wallet.amount >= initial_x @CustomError::InadequateBalance
    )]
    authority_x_wallet: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = authority_y_wallet.owner == authority.key(),
        constraint = authority_y_wallet.mint == mint_y.key(),
        constraint = authority_y_wallet.amount >= initial_y @ CustomError::InadequateBalance
    )]
    authority_y_wallet: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = authority_lt_vault.owner == authority.key(),
        constraint = authority_lt_vault.mint == lt_mint.key()
    )]
    authority_lt_vault: Account<'info, TokenAccount>,

    /// Stores pool state
    #[account(
        init,
        seeds = [torrent.key().as_ref(), mint_x.key().as_ref(), mint_y.key().as_ref()],
        bump,
        payer = authority,
        space = 4 + Pool::SIZE,
    )]
    pool_state: Account<'info, Pool>,

    /// Vault for storing x_tokens
    #[account(
        init,
        seeds = [b"x_vault".as_ref(), pool_state.key().as_ref()],
        bump,
        payer = authority,
        token::mint = mint_x,
        token::authority = pool_state
    )]
    x_token_vault: Account<'info, TokenAccount>,

    /// Vault for storing y_tokens
    #[account(
        init,
        seeds = [b"y_vault".as_ref(), pool_state.key().as_ref()],
        bump,
        payer = authority,
        token::mint = mint_y,
        token::authority = pool_state
    )]
    y_token_vault: Account<'info, TokenAccount>,

    /// System accounts
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[account]
#[derive(Default)]
pub struct Torrent {
    // Authority
    authority: Pubkey,

    // Liquidity token mint
    lt_mint: Pubkey,

    // Total supply of liquidity tokens
    total_lt_supply: u64,

    // Associated Pools
    pools: [Pubkey; MAX_POOLS],
}

impl Torrent {
    const SIZE: usize = 32 + 32 + 8 + (32 * 10);
}

#[account]
#[derive(Copy, Default)]
pub struct Pool {
    // The torrent this pool belongs to
    torrent: Pubkey,

    // lt_tokens minted by this pool
    lt_minted: u64,
}

impl Pool {
    pub const SIZE: usize = 32 + 8;
}

#[error_code]
pub enum CustomError {
    #[msg("Cost exceeds funds in wallet")]
    InadequateBalance,
}
