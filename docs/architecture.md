# Architecture

This document explains the escrow engine as a backend architecture translated into Solana accounts, instructions, and PDAs.

## System View

```mermaid
flowchart TD
    U[User Wallets] --> C[Client / Frontend]
    C --> P[Solana Program: Solia Escrow Engine]
    P --> T[Trade Account]
    P --> G[Global State PDA]
    P --> F[Frozen User PDA]
    P --> V[Vault Authority PDA]
    V --> TA[Escrow Vault Token Account]
```

## Web2 Mapping

| Web2 backend component | Solana equivalent in this repo |
| --- | --- |
| Database trade row | `Trade` account |
| Global config table | `GlobalState` PDA |
| Risk / suspension table | `FrozenUser` PDA |
| Backend business rules | Anchor instructions |
| Custodial wallet | PDA-controlled vault token account |
| Admin operations panel | Admin-only instructions |

## Core Accounts

### Trade

Represents a single escrow workflow.

Important fields:

- `initiator`
- `counterparty`
- `amount`
- `trade_type`
- `status`
- `mint`
- `payment_chain`
- `payment_token`
- `payment_wallet`
- `payment_txid`
- completion flags
- dispute metadata

### GlobalState PDA

Seed:

```text
["global_state"]
```

Stores:

- admin wallet
- pause status
- fee settings
- admin action counter

### Vault Authority PDA

Seed:

```text
["vault-authority"]
```

This PDA signs token movements out of escrow vault token accounts.

### FrozenUser PDA

Seed:

```text
["frozen_user", user_pubkey]
```

Stores whether a user is frozen from participating in trades.

## Trade Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Pending
    Pending --> Accepted: accept_trade
    Pending --> Cancelled: cancel_trade / auto_cancel
    Accepted --> Completed: mark_completed / seller_confirm_received
    Accepted --> Disputed: auto_dispute
    Disputed --> Resolved: resolve_dispute / admin_force_close
```

## Token Custody Flow

### Sell order

```mermaid
sequenceDiagram
    participant Seller
    participant Program
    participant Vault as Vault Token Account
    participant Buyer

    Seller->>Program: create_trade(sell)
    Program->>Vault: move seller tokens into escrow
    Buyer->>Program: accept_trade()
    Seller->>Program: mark_completed()
    Buyer->>Program: mark_completed()
    Program->>Buyer: release escrowed tokens
```

### Buy order

```mermaid
sequenceDiagram
    participant Buyer
    participant Program
    participant Seller
    participant Vault as Vault Token Account

    Buyer->>Program: create_trade(buy)
    Seller->>Program: accept_trade()
    Program->>Vault: move seller tokens into escrow
    Buyer->>Program: mark_completed()
    Seller->>Program: mark_completed()
    Program->>Buyer: release escrowed tokens
```

## Why This Design Fits the Bounty

This is a traditional backend pattern rebuilt as an on-chain backend:

- the state machine lives in accounts
- permission checks live in the program
- custody lives in a PDA vault
- clients only submit signed transactions

That is the exact Web2-to-Solana translation the challenge is asking for.
