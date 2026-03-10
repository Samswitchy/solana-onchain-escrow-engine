use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("J9GcXnuwFQZqpA7rSXSt44Dt4zhtyZ1RQPZdSYfXWkpt");

const TRADE_EXPIRY_SECONDS: i64 = 30 * 60;
const MAX_PAYMENT_TOKEN_LEN: usize = 32;
const MAX_PAYMENT_WALLET_LEN: usize = 128;
const MAX_PAYMENT_TXID_LEN: usize = 128;
const MAX_REASON_LEN: usize = 128;
const MAX_TRADE_SIZE: usize = 1024;
const MAX_GLOBAL_STATE_SIZE: usize = 256;
const MAX_FROZEN_USER_SIZE: usize = 256;

#[program]
pub mod p2p {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        msg!("Solia escrow engine initialized");
        Ok(())
    }

    pub fn initialize_global_state(
        ctx: Context<InitializeGlobalState>,
        fee_wallet: Pubkey,
    ) -> Result<()> {
        let global_state = &mut ctx.accounts.global_state;
        global_state.is_paused = false;
        global_state.admin = ctx.accounts.admin.key();
        global_state.paused_at = 0;
        global_state.pause_reason = String::new();
        global_state.admin_action_count = 0;
        global_state.fee_enabled = false;
        global_state.fee_bps = 0;
        global_state.fee_wallet = fee_wallet;
        global_state.min_fee_amount = 0;
        Ok(())
    }

    pub fn create_trade(
        ctx: Context<CreateTrade>,
        amount: u64,
        trade_type: TradeType,
        payment_chain: u8,
        payment_token: String,
        payment_wallet: String,
        expected_payment_amount: u64,
    ) -> Result<()> {
        require!(!ctx.accounts.global_state.is_paused, EscrowError::SystemPaused);
        assert_not_frozen(&ctx.accounts.frozen_status)?;
        validate_payment_fields(&payment_token, &payment_wallet)?;
        require!(amount > 0, EscrowError::InvalidPaymentMethod);
        require!(
            expected_payment_amount > 0,
            EscrowError::InvalidPaymentMethod
        );

        let clock = Clock::get()?;
        let trade = &mut ctx.accounts.trade;
        trade.initiator = ctx.accounts.initiator.key();
        trade.counterparty = Pubkey::default();
        trade.amount = amount;
        trade.trade_type = trade_type.clone();
        trade.status = TradeStatus::Pending;
        trade.created_at = clock.unix_timestamp;
        trade.expires_at = clock.unix_timestamp + TRADE_EXPIRY_SECONDS;
        trade.initiator_completed = false;
        trade.counterparty_completed = false;
        trade.mint = ctx.accounts.mint.key();
        trade.payment_chain = payment_chain;
        trade.payment_token = payment_token;
        trade.payment_wallet = payment_wallet;
        trade.expected_payment_amount = expected_payment_amount;
        trade.payment_txid = None;
        trade.buyer_marked_sent = false;
        trade.seller_confirmed = false;
        trade.accepted_at = 0;
        trade.payment_submitted_at = 0;
        trade.forced_closed_by = None;
        trade.forced_closed_at = 0;
        trade.admin_override_reason = None;
        trade.admin_override_outcome = None;

        if matches!(trade.trade_type, TradeType::Sell) {
            transfer_from_user_to_vault(
                &ctx.accounts.initiator_token_account,
                &ctx.accounts.vault_token_account,
                &ctx.accounts.initiator,
                &ctx.accounts.token_program,
                amount,
            )?;
        }

        emit!(TradeCreated {
            trade: trade.key(),
            initiator: trade.initiator,
            amount,
            trade_type,
            mint: trade.mint,
            created_at: trade.created_at,
        });

        Ok(())
    }

    pub fn accept_trade(ctx: Context<AcceptTrade>) -> Result<()> {
        require!(!ctx.accounts.global_state.is_paused, EscrowError::SystemPaused);
        assert_not_frozen(&ctx.accounts.frozen_status)?;

        let trade = &mut ctx.accounts.trade;
        require!(trade.status == TradeStatus::Pending, EscrowError::InvalidStatus);
        require!(
            trade.counterparty == Pubkey::default(),
            EscrowError::InvalidStatus
        );
        require!(
            ctx.accounts.counterparty.key() != trade.initiator,
            EscrowError::Unauthorized
        );

        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp <= trade.expires_at,
            EscrowError::TradeExpired
        );

        if matches!(trade.trade_type, TradeType::Buy) {
            require!(
                ctx.accounts.counterparty_token_account.owner == ctx.accounts.counterparty.key(),
                EscrowError::Unauthorized
            );
            require!(
                ctx.accounts.counterparty_token_account.mint == trade.mint,
                EscrowError::InvalidMint
            );
            transfer_from_user_to_vault(
                &ctx.accounts.counterparty_token_account,
                &ctx.accounts.vault_token_account,
                &ctx.accounts.counterparty,
                &ctx.accounts.token_program,
                trade.amount,
            )?;
        }

        trade.counterparty = ctx.accounts.counterparty.key();
        trade.status = TradeStatus::Accepted;
        trade.accepted_at = clock.unix_timestamp;

        emit!(TradeAccepted {
            trade: trade.key(),
            initiator: trade.initiator,
            counterparty: trade.counterparty,
            accepted_at: trade.accepted_at,
        });

        Ok(())
    }

    pub fn mark_completed(ctx: Context<MarkCompleted>) -> Result<()> {
        let trade = &mut ctx.accounts.trade;
        require!(trade.status == TradeStatus::Accepted, EscrowError::InvalidStatus);

        let user_key = ctx.accounts.user.key();
        if user_key == trade.initiator {
            trade.initiator_completed = true;
        } else if user_key == trade.counterparty {
            trade.counterparty_completed = true;
        } else {
            return err!(EscrowError::Unauthorized);
        }

        emit!(TradeMarkedCompleted {
            trade: trade.key(),
            user: user_key,
            initiator_completed: trade.initiator_completed,
            counterparty_completed: trade.counterparty_completed,
        });

        if trade.initiator_completed && trade.counterparty_completed {
            let recipient = trade.buyer();
            let recipient_token_account = if ctx.accounts.initiator_token_account.owner == recipient {
                &ctx.accounts.initiator_token_account
            } else {
                &ctx.accounts.counterparty_token_account
            };

            require!(
                recipient_token_account.owner == recipient,
                EscrowError::Unauthorized
            );
            require!(
                recipient_token_account.mint == trade.mint,
                EscrowError::InvalidMint
            );

            transfer_from_vault(
                &ctx.accounts.vault_token_account,
                recipient_token_account,
                &ctx.accounts.vault_authority,
                &ctx.accounts.token_program,
                trade.amount,
                ctx.bumps.vault_authority,
            )?;

            trade.status = TradeStatus::Completed;
            trade.seller_confirmed = true;

            emit!(TradeCompleted {
                trade: trade.key(),
                released_to: recipient,
                amount: trade.amount,
            });
        }

        Ok(())
    }

    pub fn cancel_trade(ctx: Context<CancelTrade>) -> Result<()> {
        let trade = &mut ctx.accounts.trade;
        require!(trade.status == TradeStatus::Pending, EscrowError::InvalidStatus);
        require!(
            ctx.accounts.initiator.key() == trade.initiator,
            EscrowError::Unauthorized
        );

        if matches!(trade.trade_type, TradeType::Sell) {
            transfer_from_vault(
                &ctx.accounts.vault_token_account,
                &ctx.accounts.initiator_token_account,
                &ctx.accounts.vault_authority,
                &ctx.accounts.token_program,
                trade.amount,
                ctx.bumps.vault_authority,
            )?;
        }

        trade.status = TradeStatus::Cancelled;
        Ok(())
    }

    pub fn auto_cancel(ctx: Context<AutoCancel>) -> Result<()> {
        let trade = &mut ctx.accounts.trade;
        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp > trade.expires_at,
            EscrowError::DeadlineNotReached
        );
        require!(trade.status == TradeStatus::Pending, EscrowError::InvalidStatus);

        if matches!(trade.trade_type, TradeType::Sell) {
            transfer_from_vault(
                &ctx.accounts.vault_token_account,
                &ctx.accounts.initiator_token_account,
                &ctx.accounts.vault_authority,
                &ctx.accounts.token_program,
                trade.amount,
                ctx.bumps.vault_authority,
            )?;
        }

        trade.status = TradeStatus::Cancelled;
        Ok(())
    }

    pub fn buyer_mark_sent(
        ctx: Context<BuyerMarkSent>,
        payment_txid: String,
    ) -> Result<()> {
        require!(
            payment_txid.len() <= MAX_PAYMENT_TXID_LEN,
            EscrowError::InvalidPaymentMethod
        );

        let trade = &mut ctx.accounts.trade;
        require!(trade.status == TradeStatus::Accepted, EscrowError::InvalidStatus);
        require!(!trade.buyer_marked_sent, EscrowError::PaymentAlreadySubmitted);
        require!(ctx.accounts.buyer.key() == trade.buyer(), EscrowError::Unauthorized);

        let clock = Clock::get()?;
        trade.payment_txid = Some(payment_txid.clone());
        trade.buyer_marked_sent = true;
        trade.payment_submitted_at = clock.unix_timestamp;

        emit!(PaymentMarkedSent {
            trade: trade.key(),
            buyer: ctx.accounts.buyer.key(),
            payment_txid,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    pub fn seller_confirm_received(ctx: Context<SellerConfirmReceived>) -> Result<()> {
        let trade = &mut ctx.accounts.trade;
        require!(trade.status == TradeStatus::Accepted, EscrowError::InvalidStatus);
        require!(trade.buyer_marked_sent, EscrowError::PaymentNotMarkedSent);
        require!(
            ctx.accounts.seller.key() == trade.seller(),
            EscrowError::Unauthorized
        );
        require!(
            ctx.accounts.buyer_token_account.owner == trade.buyer(),
            EscrowError::Unauthorized
        );

        let fee = calculate_fee(trade.amount, &ctx.accounts.global_state)?;
        let buyer_amount = trade
            .amount
            .checked_sub(fee)
            .ok_or(EscrowError::NumericalOverflow)?;

        if fee > 0 {
            require!(
                ctx.accounts.fee_receiver.owner == ctx.accounts.global_state.fee_wallet,
                EscrowError::Unauthorized
            );
            require!(
                ctx.accounts.fee_receiver.mint == trade.mint,
                EscrowError::InvalidMint
            );
            transfer_from_vault(
                &ctx.accounts.vault_token_account,
                &ctx.accounts.fee_receiver,
                &ctx.accounts.vault_authority,
                &ctx.accounts.token_program,
                fee,
                ctx.bumps.vault_authority,
            )?;
        }

        transfer_from_vault(
            &ctx.accounts.vault_token_account,
            &ctx.accounts.buyer_token_account,
            &ctx.accounts.vault_authority,
            &ctx.accounts.token_program,
            buyer_amount,
            ctx.bumps.vault_authority,
        )?;

        trade.seller_confirmed = true;
        trade.status = TradeStatus::Completed;

        emit!(PaymentConfirmed {
            trade: trade.key(),
            seller: ctx.accounts.seller.key(),
            released_to: trade.buyer(),
            amount: buyer_amount,
            fee_charged: fee,
        });

        Ok(())
    }

    pub fn auto_dispute(ctx: Context<AutoDispute>) -> Result<()> {
        let trade = &mut ctx.accounts.trade;
        require!(trade.status == TradeStatus::Accepted, EscrowError::InvalidStatus);
        require!(trade.buyer_marked_sent, EscrowError::PaymentNotMarkedSent);

        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp > trade.payment_submitted_at + TRADE_EXPIRY_SECONDS,
            EscrowError::DeadlineNotReached
        );

        trade.status = TradeStatus::Disputed;
        emit!(TradeDisputed {
            trade: trade.key(),
            caller: ctx.accounts.caller.key(),
            timestamp: clock.unix_timestamp,
        });
        Ok(())
    }

    pub fn resolve_dispute(
        ctx: Context<ResolveDispute>,
        release_to_initiator: bool,
    ) -> Result<()> {
        let trade = &mut ctx.accounts.trade;
        require!(trade.status == TradeStatus::Disputed, EscrowError::InvalidStatus);

        let destination = if release_to_initiator {
            &ctx.accounts.initiator_token_account
        } else {
            &ctx.accounts.buyer_token_account
        };

        require!(destination.mint == trade.mint, EscrowError::InvalidMint);

        transfer_from_vault(
            &ctx.accounts.vault_token_account,
            destination,
            &ctx.accounts.vault_authority,
            &ctx.accounts.token_program,
            trade.amount,
            ctx.bumps.vault_authority,
        )?;

        trade.status = TradeStatus::Resolved;
        trade.forced_closed_by = Some(ctx.accounts.admin.key());
        trade.forced_closed_at = Clock::get()?.unix_timestamp;
        trade.admin_override_reason = Some(if release_to_initiator {
            "Released to initiator".to_string()
        } else {
            "Released to buyer".to_string()
        });
        trade.admin_override_outcome = Some(if release_to_initiator { 1 } else { 0 });

        emit!(DisputeResolved {
            trade: trade.key(),
            admin: ctx.accounts.admin.key(),
            released_to_initiator: release_to_initiator,
        });

        Ok(())
    }

    pub fn admin_force_close(
        ctx: Context<AdminForceClose>,
        outcome: u8,
        reason: String,
    ) -> Result<()> {
        require!(reason.len() <= MAX_REASON_LEN, EscrowError::InvalidPaymentMethod);
        let trade = &mut ctx.accounts.trade;
        require!(
            matches!(trade.status, TradeStatus::Pending | TradeStatus::Accepted | TradeStatus::Disputed),
            EscrowError::InvalidStatus
        );

        let destination = match outcome {
            0 => &ctx.accounts.counterparty_token_account,
            1 => &ctx.accounts.initiator_token_account,
            _ => return err!(EscrowError::UnsupportedTradeType),
        };

        if ctx.accounts.vault_token_account.amount > 0 {
            transfer_from_vault(
                &ctx.accounts.vault_token_account,
                destination,
                &ctx.accounts.vault_authority,
                &ctx.accounts.token_program,
                ctx.accounts.vault_token_account.amount,
                ctx.bumps.vault_authority,
            )?;
        }

        let global_state = &mut ctx.accounts.global_state;
        global_state.admin_action_count = global_state
            .admin_action_count
            .checked_add(1)
            .ok_or(EscrowError::NumericalOverflow)?;

        trade.status = TradeStatus::Resolved;
        trade.forced_closed_by = Some(ctx.accounts.admin.key());
        trade.forced_closed_at = Clock::get()?.unix_timestamp;
        trade.admin_override_reason = Some(reason.clone());
        trade.admin_override_outcome = Some(outcome);

        emit!(TradeForceClosed {
            trade: trade.key(),
            admin: ctx.accounts.admin.key(),
            outcome,
            reason,
        });

        Ok(())
    }

    pub fn relist_trade(ctx: Context<RelistTrade>) -> Result<()> {
        let trade = &mut ctx.accounts.trade;
        require!(
            ctx.accounts.initiator.key() == trade.initiator,
            EscrowError::Unauthorized
        );
        require!(
            matches!(trade.status, TradeStatus::Cancelled | TradeStatus::Resolved),
            EscrowError::InvalidStatus
        );

        let clock = Clock::get()?;
        trade.counterparty = Pubkey::default();
        trade.status = TradeStatus::Pending;
        trade.expires_at = clock.unix_timestamp + TRADE_EXPIRY_SECONDS;
        trade.initiator_completed = false;
        trade.counterparty_completed = false;
        trade.buyer_marked_sent = false;
        trade.seller_confirmed = false;
        trade.payment_txid = None;
        trade.accepted_at = 0;
        trade.payment_submitted_at = 0;
        trade.admin_override_reason = None;
        trade.admin_override_outcome = None;
        Ok(())
    }

    pub fn set_payment_destination(
        ctx: Context<SetPaymentDestination>,
        payment_wallet: String,
    ) -> Result<()> {
        require!(
            payment_wallet.len() <= MAX_PAYMENT_WALLET_LEN,
            EscrowError::InvalidPaymentMethod
        );

        let trade = &mut ctx.accounts.trade;
        require!(
            ctx.accounts.seller.key() == trade.seller(),
            EscrowError::Unauthorized
        );
        trade.payment_wallet = payment_wallet;
        Ok(())
    }

    pub fn update_fee_config(
        ctx: Context<UpdateFeeConfig>,
        fee_enabled: bool,
        fee_bps: u16,
        fee_wallet: Pubkey,
        min_fee_amount: u64,
    ) -> Result<()> {
        require!(fee_bps <= 10_000, EscrowError::InvalidPaymentMethod);

        let global_state = &mut ctx.accounts.global_state;
        global_state.fee_enabled = fee_enabled;
        global_state.fee_bps = fee_bps;
        global_state.fee_wallet = fee_wallet;
        global_state.min_fee_amount = min_fee_amount;
        Ok(())
    }

    pub fn emergency_pause(
        ctx: Context<EmergencyPause>,
        pause: bool,
        reason: String,
    ) -> Result<()> {
        require!(reason.len() <= MAX_REASON_LEN, EscrowError::InvalidPaymentMethod);

        let global_state = &mut ctx.accounts.global_state;
        global_state.is_paused = pause;
        global_state.paused_at = Clock::get()?.unix_timestamp;
        global_state.pause_reason = reason.clone();
        global_state.admin_action_count = global_state
            .admin_action_count
            .checked_add(1)
            .ok_or(EscrowError::NumericalOverflow)?;

        if pause {
            emit!(SystemPaused {
                admin: ctx.accounts.admin.key(),
                reason,
                timestamp: global_state.paused_at,
            });
        } else {
            emit!(SystemUnpaused {
                admin: ctx.accounts.admin.key(),
                timestamp: global_state.paused_at,
            });
        }

        Ok(())
    }

    pub fn admin_freeze_user(
        ctx: Context<AdminFreezeUser>,
        freeze: bool,
        reason: String,
    ) -> Result<()> {
        require!(reason.len() <= MAX_REASON_LEN, EscrowError::InvalidPaymentMethod);

        let clock = Clock::get()?;
        let global_state = &mut ctx.accounts.global_state;
        global_state.admin_action_count = global_state
            .admin_action_count
            .checked_add(1)
            .ok_or(EscrowError::NumericalOverflow)?;

        let frozen_status = &mut ctx.accounts.frozen_status;
        frozen_status.user = ctx.accounts.target_user.key();
        frozen_status.is_frozen = freeze;
        frozen_status.frozen_at = clock.unix_timestamp;
        frozen_status.frozen_by = ctx.accounts.admin.key();
        frozen_status.reason = reason.clone();

        if freeze {
            emit!(UserFrozen {
                user: frozen_status.user,
                admin: ctx.accounts.admin.key(),
                reason,
            });
        } else {
            emit!(UserUnfrozen {
                user: frozen_status.user,
                admin: ctx.accounts.admin.key(),
            });
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
pub struct InitializeGlobalState<'info> {
    #[account(
        init,
        payer = admin,
        seeds = [b"global_state"],
        bump,
        space = 8 + MAX_GLOBAL_STATE_SIZE
    )]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateTrade<'info> {
    #[account(init, payer = initiator, space = 8 + MAX_TRADE_SIZE)]
    pub trade: Account<'info, Trade>,
    #[account(mut)]
    pub initiator: Signer<'info>,
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = initiator_token_account.owner == initiator.key() @ EscrowError::Unauthorized,
        constraint = initiator_token_account.mint == mint.key() @ EscrowError::InvalidMint
    )]
    pub initiator_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault_authority.key() @ EscrowError::InvalidVaultAuthority,
        constraint = vault_token_account.mint == mint.key() @ EscrowError::InvalidMint
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(seeds = [b"global_state"], bump)]
    pub global_state: Account<'info, GlobalState>,
    /// CHECK: PDA is validated by address and only read if initialized.
    #[account(seeds = [b"frozen_user", initiator.key().as_ref()], bump)]
    pub frozen_status: UncheckedAccount<'info>,
    /// CHECK: PDA authority over the shared vault token account.
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    /// CHECK: Present for client compatibility.
    pub associated_token_program: UncheckedAccount<'info>,
    /// CHECK: Present for client compatibility.
    pub rent: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct AcceptTrade<'info> {
    #[account(mut)]
    pub trade: Account<'info, Trade>,
    #[account(mut)]
    pub counterparty: Signer<'info>,
    #[account(mut)]
    pub counterparty_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault_authority.key() @ EscrowError::InvalidVaultAuthority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    /// CHECK: PDA authority over the shared vault token account.
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(seeds = [b"global_state"], bump)]
    pub global_state: Account<'info, GlobalState>,
    /// CHECK: PDA is validated by address and only read if initialized.
    #[account(seeds = [b"frozen_user", counterparty.key().as_ref()], bump)]
    pub frozen_status: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    /// CHECK: Present for client compatibility.
    pub rent: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct MarkCompleted<'info> {
    #[account(mut)]
    pub trade: Account<'info, Trade>,
    pub user: Signer<'info>,
    #[account(mut)]
    pub initiator_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub counterparty_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault_authority.key() @ EscrowError::InvalidVaultAuthority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    /// CHECK: PDA authority over the shared vault token account.
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelTrade<'info> {
    #[account(mut)]
    pub trade: Account<'info, Trade>,
    pub initiator: Signer<'info>,
    /// CHECK: PDA authority over the shared vault token account.
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub initiator_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault_authority.key() @ EscrowError::InvalidVaultAuthority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AutoCancel<'info> {
    #[account(mut)]
    pub trade: Account<'info, Trade>,
    pub initiator: UncheckedAccount<'info>,
    /// CHECK: PDA authority over the shared vault token account.
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub initiator_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub counterparty_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault_authority.key() @ EscrowError::InvalidVaultAuthority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub caller: Signer<'info>,
}

#[derive(Accounts)]
pub struct AutoDispute<'info> {
    #[account(mut)]
    pub trade: Account<'info, Trade>,
    pub caller: Signer<'info>,
}

#[derive(Accounts)]
pub struct BuyerMarkSent<'info> {
    #[account(mut)]
    pub trade: Account<'info, Trade>,
    pub buyer: Signer<'info>,
}

#[derive(Accounts)]
pub struct RelistTrade<'info> {
    #[account(mut)]
    pub trade: Account<'info, Trade>,
    pub initiator: Signer<'info>,
}

#[derive(Accounts)]
pub struct SetPaymentDestination<'info> {
    #[account(mut)]
    pub trade: Account<'info, Trade>,
    pub seller: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateFeeConfig<'info> {
    #[account(mut, seeds = [b"global_state"], bump, has_one = admin)]
    pub global_state: Account<'info, GlobalState>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct EmergencyPause<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"global_state"], bump, has_one = admin)]
    pub global_state: Account<'info, GlobalState>,
    /// CHECK: Optional audit log account kept for IDL compatibility.
    #[account(mut)]
    pub admin_log: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminFreezeUser<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"global_state"], bump, has_one = admin)]
    pub global_state: Account<'info, GlobalState>,
    #[account(
        init_if_needed,
        payer = admin,
        seeds = [b"frozen_user", target_user.key().as_ref()],
        bump,
        space = 8 + MAX_FROZEN_USER_SIZE
    )]
    pub frozen_status: Account<'info, FrozenUser>,
    /// CHECK: Target user only provides the public key for the freeze record PDA.
    pub target_user: UncheckedAccount<'info>,
    /// CHECK: Optional audit log account kept for IDL compatibility.
    #[account(mut)]
    pub admin_log: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminForceClose<'info> {
    pub admin: Signer<'info>,
    #[account(mut)]
    pub trade: Account<'info, Trade>,
    #[account(mut, seeds = [b"global_state"], bump, has_one = admin)]
    pub global_state: Account<'info, GlobalState>,
    /// CHECK: PDA authority over the shared vault token account.
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub counterparty_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub initiator_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault_authority.key() @ EscrowError::InvalidVaultAuthority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    /// CHECK: Optional audit log account kept for IDL compatibility.
    #[account(mut)]
    pub admin_log: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ResolveDispute<'info> {
    #[account(mut)]
    pub trade: Account<'info, Trade>,
    pub admin: Signer<'info>,
    /// CHECK: PDA authority over the shared vault token account.
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub initiator_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault_authority.key() @ EscrowError::InvalidVaultAuthority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"global_state"], bump, has_one = admin)]
    pub global_state: Account<'info, GlobalState>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SellerConfirmReceived<'info> {
    #[account(mut)]
    pub trade: Account<'info, Trade>,
    pub seller: Signer<'info>,
    /// CHECK: Included for client compatibility and optional validation.
    pub initiator: UncheckedAccount<'info>,
    #[account(seeds = [b"global_state"], bump)]
    pub global_state: Account<'info, GlobalState>,
    #[account(mut)]
    pub fee_receiver: Account<'info, TokenAccount>,
    /// CHECK: PDA authority over the shared vault token account.
    #[account(seeds = [b"vault-authority"], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        constraint = vault_token_account.owner == vault_authority.key() @ EscrowError::InvalidVaultAuthority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub buyer_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct Trade {
    pub initiator: Pubkey,
    pub counterparty: Pubkey,
    pub amount: u64,
    pub trade_type: TradeType,
    pub status: TradeStatus,
    pub created_at: i64,
    pub expires_at: i64,
    pub initiator_completed: bool,
    pub counterparty_completed: bool,
    pub mint: Pubkey,
    pub payment_chain: u8,
    pub payment_token: String,
    pub payment_wallet: String,
    pub expected_payment_amount: u64,
    pub payment_txid: Option<String>,
    pub buyer_marked_sent: bool,
    pub seller_confirmed: bool,
    pub accepted_at: i64,
    pub payment_submitted_at: i64,
    pub forced_closed_by: Option<Pubkey>,
    pub forced_closed_at: i64,
    pub admin_override_reason: Option<String>,
    pub admin_override_outcome: Option<u8>,
}

impl Trade {
    fn buyer(&self) -> Pubkey {
        match self.trade_type {
            TradeType::Buy => self.initiator,
            TradeType::Sell => self.counterparty,
        }
    }

    fn seller(&self) -> Pubkey {
        match self.trade_type {
            TradeType::Buy => self.counterparty,
            TradeType::Sell => self.initiator,
        }
    }
}

#[account]
pub struct GlobalState {
    pub is_paused: bool,
    pub admin: Pubkey,
    pub paused_at: i64,
    pub pause_reason: String,
    pub admin_action_count: u64,
    pub fee_enabled: bool,
    pub fee_bps: u16,
    pub fee_wallet: Pubkey,
    pub min_fee_amount: u64,
}

#[account]
pub struct FrozenUser {
    pub user: Pubkey,
    pub is_frozen: bool,
    pub frozen_at: i64,
    pub frozen_by: Pubkey,
    pub reason: String,
}

#[account]
pub struct AdminLog {
    pub admin_id: Pubkey,
    pub action: AdminActionType,
    pub timestamp: i64,
    pub reason: String,
    pub target: Pubkey,
    pub log_index: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum TradeType {
    Buy,
    Sell,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum TradeStatus {
    Pending,
    Accepted,
    Completed,
    Disputed,
    Resolved,
    Cancelled,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum AdminActionType {
    FreezeUser,
    UnfreezeUser,
    PauseSystem,
    UnpauseSystem,
    ForceCloseTrade,
    UpdateFeeConfig,
}

#[event]
pub struct TradeCreated {
    pub trade: Pubkey,
    pub initiator: Pubkey,
    pub amount: u64,
    pub trade_type: TradeType,
    pub mint: Pubkey,
    pub created_at: i64,
}

#[event]
pub struct TradeAccepted {
    pub trade: Pubkey,
    pub initiator: Pubkey,
    pub counterparty: Pubkey,
    pub accepted_at: i64,
}

#[event]
pub struct TradeMarkedCompleted {
    pub trade: Pubkey,
    pub user: Pubkey,
    pub initiator_completed: bool,
    pub counterparty_completed: bool,
}

#[event]
pub struct TradeCompleted {
    pub trade: Pubkey,
    pub released_to: Pubkey,
    pub amount: u64,
}

#[event]
pub struct PaymentMarkedSent {
    pub trade: Pubkey,
    pub buyer: Pubkey,
    pub payment_txid: String,
    pub timestamp: i64,
}

#[event]
pub struct PaymentConfirmed {
    pub trade: Pubkey,
    pub seller: Pubkey,
    pub released_to: Pubkey,
    pub amount: u64,
    pub fee_charged: u64,
}

#[event]
pub struct TradeDisputed {
    pub trade: Pubkey,
    pub caller: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct DisputeResolved {
    pub trade: Pubkey,
    pub admin: Pubkey,
    pub released_to_initiator: bool,
}

#[event]
pub struct TradeForceClosed {
    pub trade: Pubkey,
    pub admin: Pubkey,
    pub outcome: u8,
    pub reason: String,
}

#[event]
pub struct UserFrozen {
    pub user: Pubkey,
    pub admin: Pubkey,
    pub reason: String,
}

#[event]
pub struct UserUnfrozen {
    pub user: Pubkey,
    pub admin: Pubkey,
}

#[event]
pub struct SystemPaused {
    pub admin: Pubkey,
    pub reason: String,
    pub timestamp: i64,
}

#[event]
pub struct SystemUnpaused {
    pub admin: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct AdminActionLogged {
    pub admin_id: Pubkey,
    pub action: AdminActionType,
    pub timestamp: i64,
    pub reason: String,
    pub target: Pubkey,
    pub log_index: u64,
}

#[event]
pub struct DisputeOverridden {
    pub trade: Pubkey,
    pub admin: Pubkey,
    pub outcome: u8,
    pub reason: String,
}

#[error_code]
pub enum EscrowError {
    #[msg("The trade has expired")]
    TradeExpired,
    #[msg("The trade is in an invalid status for this action")]
    InvalidStatus,
    #[msg("The caller is not authorized for this action")]
    Unauthorized,
    #[msg("The admin signer is not authorized")]
    UnauthorizedAdmin,
    #[msg("Buyer payment has not been marked as sent")]
    PaymentNotMarkedSent,
    #[msg("Vault authority does not match the expected PDA")]
    InvalidVaultAuthority,
    #[msg("Payment proof was already submitted")]
    PaymentAlreadySubmitted,
    #[msg("The deadline has not been reached yet")]
    DeadlineNotReached,
    #[msg("The system is currently paused")]
    SystemPaused,
    #[msg("The user is frozen and cannot interact with the program")]
    UserFrozen,
    #[msg("The payment method or parameters are invalid")]
    InvalidPaymentMethod,
    #[msg("This trade type or admin outcome is unsupported")]
    UnsupportedTradeType,
    #[msg("A numerical overflow occurred")]
    NumericalOverflow,
    #[msg("The provided token mint is invalid")]
    InvalidMint,
    #[msg("A payment destination is required")]
    PaymentDestinationMissing,
}

fn assert_not_frozen(account: &UncheckedAccount<'_>) -> Result<()> {
    if account.data_is_empty() {
        return Ok(());
    }

    let mut data: &[u8] = &account.try_borrow_data()?;
    let frozen_user = FrozenUser::try_deserialize(&mut data)?;
    require!(!frozen_user.is_frozen, EscrowError::UserFrozen);
    Ok(())
}

fn validate_payment_fields(payment_token: &str, payment_wallet: &str) -> Result<()> {
    require!(
        !payment_token.is_empty() && payment_token.len() <= MAX_PAYMENT_TOKEN_LEN,
        EscrowError::InvalidPaymentMethod
    );
    require!(
        !payment_wallet.is_empty() && payment_wallet.len() <= MAX_PAYMENT_WALLET_LEN,
        EscrowError::PaymentDestinationMissing
    );
    Ok(())
}

fn transfer_from_user_to_vault<'info>(
    from: &Account<'info, TokenAccount>,
    to: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = Transfer {
        from: from.to_account_info(),
        to: to.to_account_info(),
        authority: authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)
}

fn transfer_from_vault<'info>(
    from: &Account<'info, TokenAccount>,
    to: &Account<'info, TokenAccount>,
    authority: &UncheckedAccount<'info>,
    token_program: &Program<'info, Token>,
    amount: u64,
    bump: u8,
) -> Result<()> {
    let signer_seeds: &[&[u8]] = &[b"vault-authority", &[bump]];
    let signer = [signer_seeds];
    let cpi_accounts = Transfer {
        from: from.to_account_info(),
        to: to.to_account_info(),
        authority: authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        cpi_accounts,
        &signer,
    );
    token::transfer(cpi_ctx, amount)
}

fn calculate_fee(amount: u64, global_state: &GlobalState) -> Result<u64> {
    if !global_state.fee_enabled || amount == 0 {
        return Ok(0);
    }

    let bps_fee = amount
        .checked_mul(global_state.fee_bps as u64)
        .ok_or(EscrowError::NumericalOverflow)?
        .checked_div(10_000)
        .ok_or(EscrowError::NumericalOverflow)?;

    let fee = bps_fee.max(global_state.min_fee_amount);
    Ok(fee.min(amount))
}
