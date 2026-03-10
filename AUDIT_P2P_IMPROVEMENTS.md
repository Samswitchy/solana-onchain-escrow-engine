# Escrow Engine Audit (Draft)

## Scope
- `programs/p2p/src/*`
- `tests/p2p.js`
- `Anchor.toml`

## Summary
- Critical issues are present in admin authorization, global state initialization, and vault authority usage.
- Multiple logic mismatches block buy-type trades and allow trade role confusion.
- Test suite does not match the current program interface and account model.
## Work Completed (So Far)
- Admin authorization hardened in admin contexts (`has_one = admin`) and resolve-dispute now checks `global_state.admin`.
- Vault authority enforced across contexts and used correctly in CPI transfers.
- Frozen-user checks now use optional accounts wired into contexts.
- Input validation tightened for trade creation and payment txid length.
- Trade relist restricted to pending trades and resets acceptance/completion state; create-trade initializes acceptance timestamps.
- Buyer payment marking now requires Accepted status.
- Tests updated to current instruction signature, global vault model, and unsupported buy flow.
- Tests: `anchor test --provider.cluster localnet` (12 passing).

## Findings (Critical)
1) Admin authorization is missing for admin-only instructions.
   - Any signer can freeze users, force close trades, or pause the system.
   - Affected: `programs/p2p/src/contexts/admin_freeze_user_ctx.rs`, `programs/p2p/src/contexts/admin_force_close_ctx.rs`, `programs/p2p/src/contexts/emergency_pause_ctx.rs`.
   - Recommendation: Add `has_one = admin` on `global_state` and/or `require_keys_eq!(admin.key(), global_state.admin, ...)` to all admin instructions.

2) Global state cannot be initialized because ADMIN_PUBKEY is the program ID.
   - `ADMIN_PUBKEY` comes from `declare_id!()` in `programs/p2p/src/constants.rs`, which is the program id, not an admin wallet.
   - `initialize_global_state` requires `admin.key() == ADMIN_PUBKEY`, which no user can satisfy.
   - Recommendation: Replace `ADMIN_PUBKEY` with an actual admin key constant or use a passed-in admin key and store it in `GlobalState`.

3) Vault authority is not enforced and `mark_completed` uses the wrong authority.
   - `mark_completed` signs with the vault authority PDA but sets `authority` to the vault token account, not the PDA.
   - No constraints ensure `vault_token_account.owner == vault_authority` in multiple contexts.
   - Affected: `programs/p2p/src/instructions/mark_completed.rs`, `programs/p2p/src/contexts/mark_completed_ctx.rs`, `programs/p2p/src/contexts/create_trade_ctx.rs`, `programs/p2p/src/contexts/cancel_trade_ctx.rs`, `programs/p2p/src/contexts/auto_cancel_ctx.rs`, `programs/p2p/src/contexts/resolve_dispute_ctx.rs`, `programs/p2p/src/contexts/admin_force_close_ctx.rs`, `programs/p2p/src/contexts/seller_confirm_received_ctx.rs`.
   - Recommendation: Add a `vault_authority` account to contexts that transfer from the vault and constrain the vault token account’s owner to that PDA. Use the PDA as the `authority` in CPI transfers.

## Findings (High)
4) Frozen user checks do not work as implemented.
   - The code only inspects `ctx.accounts` and does not use `remaining_accounts`, so the frozen account is never found.
   - Affected: `programs/p2p/src/instructions/create_trade.rs`, `programs/p2p/src/instructions/accept_trade.rs`.
   - Recommendation: Add the frozen user account to the context or accept it in `remaining_accounts` and explicitly read it.

5) Trade role logic breaks buy-type trades.
   - `buyer_mark_sent` requires `trade.counterparty == buyer`, but in buy trades the buyer is the initiator.
   - `seller_confirm_received` requires `trade.initiator == seller`, which fails for buy trades.
   - `auto_cancel` and related logic assume initiator is always seller.
   - Affected: `programs/p2p/src/contexts/buyer_mark_sent_ctx.rs`, `programs/p2p/src/contexts/seller_confirm_received_ctx.rs`, `programs/p2p/src/instructions/auto_cancel.rs`.
   - Recommendation: Tie role checks to `trade.trade_type` and define consistent buyer/seller roles per trade type.

## Findings (Medium)
6) Input length checks do not match allocated account sizes.
   - `payment_wallet` is limited to 48 chars, but storage allows 68.
   - `payment_txid` length is not checked at all (can exceed allocated space).
   - Affected: `programs/p2p/src/instructions/create_trade.rs`, `programs/p2p/src/instructions/buyer_mark_sent.rs`, `programs/p2p/src/state/trade.rs`.
   - Recommendation: Enforce length constraints that match `Trade::MAX_*` constants.

7) Admin identity checks are inconsistent across the program.
   - `ADMIN_WALLET` is used in `resolve_dispute`, while `initialize_global_state` uses `ADMIN_PUBKEY`.
   - `update_fee_config` only enforces `has_one = admin` on `global_state` but does not check `admin` against a canonical constant.
   - Affected: `programs/p2p/src/constants.rs`, `programs/p2p/src/lib.rs`, `programs/p2p/src/instructions/resolve_dispute.rs`, `programs/p2p/src/contexts/update_fee_config_ctx.rs`.
   - Recommendation: Use a single admin authority source: `global_state.admin`.

8) `relist_trade` does not reset all acceptance state.
   - `counterparty`, `accepted_at`, and `payment_submitted_at` are not cleared.
   - Affected: `programs/p2p/src/instructions/relist_trade.rs`.
   - Recommendation: Clear those fields and reinitialize any deadlines.
   - Status: fixed (pending-only relist; completion flags reset).

## Findings (Low)
9) Trade amount and expected payment amount are not validated to be > 0.
   - Affected: `programs/p2p/src/instructions/create_trade.rs`.
   - Recommendation: Require nonzero values.

10) Tests are out of sync with program interface and account model.
   - `create_trade` args and accounts do not match current instruction signature or global vault model.
   - `dispute_trade` is referenced but does not exist in the program.
   - Affected: `tests/p2p.js`.
   - Recommendation: Update tests to use `vault_authority`/`vault_token_account` and current instruction parameters.

## Improvements Checklist
- Define and enforce a single admin authority (`global_state.admin`). (done)
- Add `vault_authority` to contexts and validate `vault_token_account.owner`. (done)
- Fix role checks for buy trades and clarify trade lifecycle by trade type. (done)
- Add strict length checks for all variable-length fields. (partial: payment_wallet/token/txid enforced)
- Align tests and frontend integration with current instruction interfaces. (done: tests updated)
- Add coverage for admin actions, freeze logic, and failure modes. (partial: core paths covered; disputes pending)

## Notes
- Frontend integration files were not found under `frontend/src` in this repo snapshot.


Medium: Fee config is unbounded; fee_bps can be > 10_000 and min_fee_amount can exceed trade.amount, which will revert at payout. Add validation in the instruction to keep fees sane. See update_fee_config.rs (line 4).

High: mark_completed is still callable and releases escrow solely based on both parties signing, bypassing the off‑chain payment proof flow (buyer_mark_sent/seller_confirm_received). That undermines the escrow workflow if exposed in production. See mark_completed.rs (line 10).



Remove or hard‑gate mark_completed.
Add validation in update_fee_config.
Make frozen-user account required or enforce via PDA constraint.
Add PDA seeds constraint to UpdateFeeConfig context.
