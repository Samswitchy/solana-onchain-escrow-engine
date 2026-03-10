# Solia App Feature Roadmap

This is the app-facing roadmap for Solia.
It sits next to the smart contract roadmap in this repo so the product direction is visible in one place.

Use this file for:
- app structure
- feature surfaces
- UX transitions
- escrow hub rollout
- mobile and web feature parity
- backend-backed product behavior outside direct contract scope

Use `product_roadmap.md` for:
- on-chain architecture
- account models
- instruction design
- contract lifecycle enforcement

---

## Status Legend

- ✅ Implemented
- 🚧 In Progress
- ⏳ Planned
- 📌 Recommendation

---

## Current Direction

Solia is moving from a `P2P marketplace` product to an `escrow infrastructure` product.

User-facing structure:
- `Send`
- `Receive`
- `Escrow`

Within `Escrow`:
- `Marketplace`
- `Private Link`
- `Service Escrow`

This is the right direction because:
- `P2P` sounds narrow and marketplace-only
- `Escrow` positions Solia as trust infrastructure
- it creates room for multiple protected transaction flows under one product

---

## What Is Already Done

### ✅ Marketplace Trust + Profile Foundation

- Username rules and wallet-linked identity are implemented
- Display name and avatar support are implemented
- Joined date and improved identity display are implemented
- Trade cards now show better identity surfaces instead of raw wallet-only presentation
- Sensitive trade actions now use wallet-signed flows

### ✅ Seller Payment Rails

- Seller payment methods are implemented
- Payment details become visible after trade acceptance
- Seller payment method validation is implemented
- Payment details can auto-fill into relevant trade flows

### ✅ Safety + Moderation Foundation

- Chat filtering blocks risky content such as phone numbers and external links
- Payment instruction filtering is implemented
- Auto-flagging and warning behavior is implemented
- Freeze and unfreeze admin actions are implemented

### ✅ Admin Operations

- Admin user search and profile review are implemented
- Verification actions and related audit behavior are implemented
- Admin settings for verification thresholds are implemented
- User profile side-panel and moderation visibility are implemented

### ✅ Analytics + Revenue Foundation

- Event logging exists for trade and moderation flows
- Admin analytics viewer exists
- CSV export exists
- KPI summaries exist
- Payment and release timing metrics are implemented
- Revenue analytics foundation is implemented

### ✅ Escrow Transition Foundation

- Home shortcut rename to `Escrow` is implemented
- Escrow Hub screen is implemented
- `Marketplace` entry is wired
- Placeholder destinations for `Private Link` and `Service Escrow` exist
- `/escrow` is the primary route and `/p2p` remains a compatibility alias

---

## Phase 1 - Escrow Hub

### ✅ Implemented

- Rename the main user-facing shortcut from `P2P` to `Escrow`
- Add the `Escrow Hub` landing surface
- Add three entry cards:
  - `Marketplace`
  - `Private Link`
  - `Service Escrow`
- Route `Marketplace` to the current marketplace flow
- Keep placeholder destinations for non-live escrow modes
- Move escrow hub feature modules under a dedicated escrow-oriented structure

### 📌 Recommendation

- Keep internal naming stable until the escrow information architecture is fully settled
- Avoid deep folder renames that create churn without product benefit

---

## Phase 2 - Marketplace Cleanup

### ✅ Implemented

- User-facing `P2P` naming has been largely shifted to `Escrow` / `Marketplace`
- Primary route behavior is aligned with the escrow-first structure
- Marketplace remains functional inside the new Escrow Hub
- Core buy/sell marketplace behavior is preserved

### 🚧 Remaining

- Final cleanup of legacy strings and comments
- Final wording pass so mobile and web labels are fully aligned

---

## Phase 3 - Private Link MVP

### ✅ Implemented

- Mobile create/deals flow exists
- Private escrow records persist to backend
- Shareable link generation exists
- Web receiver route exists
- History views use real backend data
- Intent-based UX is implemented:
  - `I'm Paying`
  - `I'm Receiving`
- Proof note and proof link fields exist
- Receiver flow reflects funding direction
- Hybrid dual-confirmation flow exists:
  - first confirmation moves to awaiting both confirmations
  - second confirmation completes
  - timeout moves to dispute

### 🚧 Current Limitation

- Funding, release, and dispute are still backend-backed transitions
- Private link is not yet fully enforced by on-chain escrow

### 📌 Recommendation

- Keep the current backend-backed UX
- Move actual custody and release enforcement into the contract layer next

---

## Phase 3A - Private Link Smart Contract Layer

### ⏳ Planned

- Add dedicated on-chain `PrivateEscrow` support in the shared Solia program
- Add instruction surface for:
  - `create_private_escrow`
  - `fund_private_escrow`
  - `release_private_escrow`
  - `cancel_private_escrow`
  - `dispute_private_escrow`
- Link backend records to on-chain escrow IDs
- Replace backend-only lifecycle transitions with real contract-enforced escrow

### 📌 Recommendation

- Do not model private link as a stretched version of marketplace `Trade`
- Keep it under the same program, but give it its own account model

---

## Phase 4 - Service Escrow MVP

### 🚧 Planned / Early Direction

- Add service escrow create flow
- Add service deals management view
- Add status filtering for service deals
- Add deal summary and delivery confirmation
- Add release, cancel, and dispute entry points
- Support lightweight mock lifecycle actions first where necessary

### 📌 Recommendation

- Treat service escrow as a first-class product surface
- Do not reuse marketplace role naming blindly for service flows

---

## Phase 5 - Unified Escrow History

### ⏳ Planned

- Create a shared history surface across:
  - marketplace
  - private links
  - service escrow
- Standardize transaction status language
- Surface disputes consistently across all escrow modes

---

## Phase 6 - Activity And Notifications Separation

### ⏳ Planned

- Keep transaction and escrow events inside recent activity
- Keep security and routine system alerts inside the notification inbox
- Prevent recent activity from becoming a mixed feed of unrelated events

---

## Marketplace Mobile Parity

### ✅ Implemented

- Browse ads now use real backend data
- Market cards prefer username and display name where possible
- Listing detail to accept-trade to trade-room flow is implemented
- Accept-trade flow performs wallet signing and backend logging
- Trade room has a live message/chat surface
- `My Ads` reflects real backend statuses more clearly
- Active trade limits and cooldown states now surface in app flows
- Settlement choices are clearer:
  - `Crypto Settle`
  - `Bank Settle`
- Bank-only fiat selection is enforced when relevant
- Real min/max limits and pricing now display correctly

### 🚧 Remaining

- Partial-fill acceptance is not implemented
- Trade room still needs full role-based parity with web for:
  - `Mark Payment Sent`
  - `Release Funds`
  - `Open Dispute`
- One final label and settlement-copy pass is still needed

---

## Analytics + Revenue

### ✅ Implemented

- Admin analytics summary KPIs
- Event export
- Relative and UTC timestamps
- Payment time and release time KPIs
- Reputation timing metrics
- Revenue tab
- Total traded volume KPI
- Rail count and rail volume breakdowns
- Fiat volume selector
- Most traded rail by count
- Top rail by volume

### ⏳ Remaining

- Final fee KPI wording
- Reliable fee data on completed trades once fees are enabled
- Historical fee backfill if needed
- KPI validation against production records
- Deeper charts only after KPI accuracy is confirmed

---

## Phase 9 - Escrow Infrastructure Transition

### 🚧 In Progress

- Reposition Solia from `P2P marketplace` to `escrow infrastructure`
- Keep `Marketplace` as one mode inside `Escrow`, not the entire product
- Preserve one shared escrow core while expanding the product surface area

### ✅ Already Implemented Toward This Goal

- Escrow Hub exists
- Marketplace is already nested under Escrow positioning
- Compatibility routing has been preserved
- Product language has started shifting from `P2P` to `Escrow` / `Marketplace`

### ⏳ Next Expansion

- Finish marketplace wording cleanup
- Add stronger private escrow product surface
- Add service escrow product surface
- Keep backend, app, and contract terminology aligned

---

## Technical Product Direction

Suggested long-term app structure:

```text
lib/tabs/escrow_hub/
  p2p/
  private_link/
    models/
    services/
    screens/
    widgets/
  service_escrow/
  widgets/
```

Suggested shared product model:
- `EscrowTransaction`
- `EscrowType`
- `EscrowStatus`
- `EscrowParty`
- `EscrowFundingMethod`

Recommended escrow types:
- `marketplace`
- `private_link`
- `service`

---

## What I Think

The transition is correct.

The strongest product decision is this:
- keep `Marketplace` alive
- stop making it the whole brand

That gives Solia more room to grow into a trust product instead of staying boxed in as a trading feature.

The next important discipline is boundary clarity:
- app roadmap decides surfaces and user flows
- contract roadmap decides enforcement and custody

If those two stay aligned, the product can expand cleanly.
If they drift, you will get UX promises that the contract does not actually enforce.

---

## Immediate Recommended Next Steps

1. Finish the last marketplace wording cleanup across app surfaces.
2. Lock the shared terminology for `Marketplace`, `Private Link`, and `Service Escrow`.
3. Design the `PrivateEscrow` contract model before adding more private-link UX complexity.
4. Keep Service Escrow at product-design level until Private Link has a stable contract path.
