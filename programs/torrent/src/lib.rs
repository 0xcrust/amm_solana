use anchor_lang::prelude::*;
use anchor_spl::token::{Burn, Mint, MintTo, Token, TokenAccount, Transfer};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub const MAX_POOLS: usize = 10;

/// Wherever it's used, lt stands for liquidity token.
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
        let torrent = &mut ctx.accounts.torrent;
        let pool = &mut ctx.accounts.pool;

        let pool_index = torrent.register_pool(pool.key()).unwrap();
        pool.index = pool_index;
        pool.torrent = torrent.key();
        let mint_amount = (initial_x + initial_y) >> 1;

        pool.lt_minted = pool.lt_minted.checked_add(mint_amount).unwrap();
        torrent.total_lt_supply = torrent.total_lt_supply.checked_add(mint_amount).unwrap();

        let torrent_bump = *ctx.bumps.get("torrent").unwrap();
        let authority = ctx.accounts.authority.key();
        let torrent_signature = &[b"torrent", authority.as_ref(), &[torrent_bump]];

        anchor_spl::token::mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    to: ctx.accounts.authority_lt_wallet.to_account_info(),
                    mint: ctx.accounts.lt_mint.to_account_info(),
                    authority: ctx.accounts.torrent.to_account_info(),
                },
            )
            .with_signer(&[&torrent_signature[..]]),
            mint_amount,
        )?;

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.authority_x_wallet.to_account_info(),
                    to: ctx.accounts.x_token_vault.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            initial_x,
        )?;

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.authority_y_wallet.to_account_info(),
                    to: ctx.accounts.y_token_vault.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            initial_y,
        )?;

        Ok(())
    }

    pub fn add_liquidity(
        ctx: Context<AlterLiquidity>,
        amount_x: u64,
        _amount_y: u64,
    ) -> Result<()> {
        let x_vault_balance = ctx.accounts.x_token_vault.amount;
        let y_vault_balance = ctx.accounts.y_token_vault.amount;

        let x_deposit = amount_x;
        let y_deposit = y_vault_balance
            .checked_mul(x_deposit)
            .unwrap()
            .checked_div(x_vault_balance)
            .unwrap();

        let user_x_balance = ctx.accounts.user_x_wallet.amount;
        let user_y_balance = ctx.accounts.user_y_wallet.amount;
        require!(user_x_balance >= x_deposit, CustomError::InadequateBalance);
        require!(user_y_balance >= y_deposit, CustomError::InadequateBalance);

        let pool_lt_minted = ctx.accounts.pool.lt_minted;

        // Does this work? Looks like multiplication is guaranteed to fail.
        // Test on code completion and apply fix if needed.
        let mint_amount = y_deposit
            .checked_mul(pool_lt_minted)
            .unwrap()
            .checked_div(y_vault_balance)
            .unwrap();

        let pool = &mut ctx.accounts.pool;
        let torrent = &mut ctx.accounts.torrent;
        pool.lt_minted = pool.lt_minted.checked_add(mint_amount).unwrap();
        torrent.total_lt_supply = torrent.total_lt_supply.checked_add(mint_amount).unwrap();

        let torrent_bump = *ctx.bumps.get("torrent").unwrap();
        let torrent_authority = torrent.authority;
        let torrent_signature = &[b"torrent", torrent_authority.as_ref(), &[torrent_bump]];

        anchor_spl::token::mint_to(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    to: ctx.accounts.user_lt_wallet.to_account_info(),
                    mint: ctx.accounts.lt_mint.to_account_info(),
                    authority: ctx.accounts.torrent.to_account_info(),
                },
            )
            .with_signer(&[&torrent_signature[..]]),
            mint_amount,
        )?;

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_x_wallet.to_account_info(),
                    to: ctx.accounts.x_token_vault.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            x_deposit,
        )?;

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_y_wallet.to_account_info(),
                    to: ctx.accounts.y_token_vault.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            y_deposit,
        )?;

        Ok(())
    }

    pub fn subtract_liquidity(ctx: Context<AlterLiquidity>, lt_amount: u64) -> Result<()> {
        let user_lt_balance = ctx.accounts.user_lt_wallet.amount;
        require!(user_lt_balance >= lt_amount, CustomError::InadequateBalance);
        let pool_lt_minted = ctx.accounts.pool.lt_minted;
        require!(pool_lt_minted >= lt_amount, CustomError::ExcessiveBurn);

        let x_vault_balance = ctx.accounts.x_token_vault.amount;
        let y_vault_balance = ctx.accounts.y_token_vault.amount;

        // Does this work?
        let x_owed = lt_amount
            .checked_mul(x_vault_balance)
            .unwrap()
            .checked_div(pool_lt_minted)
            .unwrap();
        let y_owed = lt_amount
            .checked_mul(y_vault_balance)
            .unwrap()
            .checked_div(pool_lt_minted)
            .unwrap();

        let pool = &mut ctx.accounts.pool;
        let torrent = &mut ctx.accounts.torrent;

        pool.lt_minted = pool.lt_minted.checked_sub(lt_amount).unwrap();
        torrent.total_lt_supply = torrent.total_lt_supply.checked_sub(lt_amount).unwrap();

        let pool_bump = *ctx.bumps.get("pool").unwrap();
        let x_token_mint = ctx.accounts.x_token_vault.mint;
        let y_token_mint = ctx.accounts.y_token_vault.mint;
        let torrent_key = torrent.key();

        let pool_signature = &[
            torrent_key.as_ref(),
            x_token_mint.as_ref(),
            y_token_mint.as_ref(),
            &[pool_bump],
        ];

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.x_token_vault.to_account_info(),
                    to: ctx.accounts.user_x_wallet.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
            )
            .with_signer(&[&pool_signature[..]]),
            x_owed,
        )?;

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.x_token_vault.to_account_info(),
                    to: ctx.accounts.user_x_wallet.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
            )
            .with_signer(&[&pool_signature[..]]),
            y_owed,
        )?;

        anchor_spl::token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Burn {
                    mint: ctx.accounts.lt_mint.to_account_info(),
                    from: ctx.accounts.user_lt_wallet.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            lt_amount,
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
    torrent: Box<Account<'info, Torrent>>,
    lt_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    authority: Signer<'info>,

    /// Token pair for this pool
    mint_x: Box<Account<'info, Mint>>,
    mint_y: Box<Account<'info, Mint>>,

    /// Authority's token accounts
    #[account(
        mut,
        constraint = authority_x_wallet.owner == authority.key(),
        constraint = authority_x_wallet.mint == mint_x.key(),
        constraint = authority_x_wallet.amount >= initial_x @CustomError::InadequateBalance
    )]
    authority_x_wallet: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = authority_y_wallet.owner == authority.key(),
        constraint = authority_y_wallet.mint == mint_y.key(),
        constraint = authority_y_wallet.amount >= initial_y @ CustomError::InadequateBalance
    )]
    authority_y_wallet: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = authority_lt_wallet.owner == authority.key(),
        constraint = authority_lt_wallet.mint == lt_mint.key()
    )]
    authority_lt_wallet: Box<Account<'info, TokenAccount>>,

    /// Stores pool state
    #[account(
        init,
        seeds = [torrent.key().as_ref(), mint_x.key().as_ref(), mint_y.key().as_ref()],
        bump,
        payer = authority,
        space = 4 + Pool::SIZE,
    )]
    pool: Box<Account<'info, Pool>>,

    /// Vault for storing x_tokens
    #[account(
        init,
        seeds = [b"x_vault".as_ref(), pool.key().as_ref()],
        bump,
        payer = authority,
        token::mint = mint_x,
        token::authority = pool
    )]
    x_token_vault: Box<Account<'info, TokenAccount>>,

    /// Vault for storing y_tokens
    #[account(
        init,
        seeds = [b"y_vault".as_ref(), pool.key().as_ref()],
        bump,
        payer = authority,
        token::mint = mint_y,
        token::authority = pool
    )]
    y_token_vault: Box<Account<'info, TokenAccount>>,

    /// System accounts
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct AlterLiquidity<'info> {
    user: Signer<'info>,

    #[account(
        has_one = lt_mint,
        constraint = torrent.pools[pool.index as usize] == pool.key() @ CustomError::TorrentPoolMismatch
    )]
    torrent: Box<Account<'info, Torrent>>,
    #[account(has_one = torrent)]
    pool: Box<Account<'info, Pool>>,

    #[account(mut, seeds = [b"x_vault".as_ref(), pool.key().as_ref()], bump)]
    x_token_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [b"y_vault".as_ref(), pool.key().as_ref()], bump)]
    y_token_vault: Box<Account<'info, TokenAccount>>,

    lt_mint: Box<Account<'info, Mint>>,

    /// User's token accounts
    #[account(
        mut,
        constraint = user_x_wallet.owner == user.key(),
        constraint = user_x_wallet.mint == x_token_vault.mint,
    )]
    user_x_wallet: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = user_y_wallet.owner == user.key(),
        constraint = user_y_wallet.mint == y_token_vault.mint,
    )]
    user_y_wallet: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = user_lt_wallet.owner == user.key(),
        constraint = user_lt_wallet.mint == lt_mint.key(),
    )]
    user_lt_wallet: Box<Account<'info, TokenAccount>>,

    token_program: Program<'info, Token>,
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

    pub fn register_pool(&mut self, new_pool: Pubkey) -> Result<u8> {
        for (index, pool) in self.pools.iter_mut().enumerate() {
            if *pool != Pubkey::default() {
                continue;
            }

            *pool = new_pool;

            return Ok(index as u8);
        }
        Err(error!(CustomError::MaxPoolLimit))
    }
}

#[account]
#[derive(Copy, Default)]
pub struct Pool {
    // Pool's position in the torrent
    index: u8,

    // The torrent this pool belongs to
    torrent: Pubkey,

    // lt_tokens minted by this pool
    lt_minted: u64,
}

impl Pool {
    pub const SIZE: usize = 1 + 32 + 8;
}

#[error_code]
pub enum CustomError {
    #[msg("Cost exceeds funds in wallet")]
    InadequateBalance,
    #[msg("No free pool in torrent")]
    MaxPoolLimit,
    #[msg("Pool does not belong to torrent")]
    TorrentPoolMismatch,
    #[msg("Burn exceeds pool limits")]
    ExcessiveBurn,
}
