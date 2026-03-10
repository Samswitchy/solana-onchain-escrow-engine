# Solia Escrow Smart Contract Roadmap

This roadmap is the contract-facing version of Solia's broader product roadmap.
It aligns this repo with the app transition from a `P2P marketplace` product to an `escrow infrastructure` product.

This file should answer one question clearly:

What should the Solana program in this repo support next?

---

## Status Legend

- ✅ Implemented
- 🚧 In Progress
- ⏳ Planned
- 📌 Recommendation

---

## Repo Scope

This repo should focus on:
- on-chain custody
- escrow lifecycle enforcement
- release and refund rules
- dispute and admin controls
- fee handling
- account models and IDL stability
- test coverage for contract behavior

This repo should not be the source of truth for:
- Flutter screen structure
- web routing
- landing page copy
- admin dashboard UX
- mobile parity tasks

Those belong in the app roadmap.

---

## Product Alignment

The app roadmap now positions Solia as:
- `Send`
- `Receive`
- `Escrow`

Within that model:
- `Marketplace` is one escrow mode
- `Private Link` is another escrow mode
- `Service Escrow` is another escrow mode

The contract implication is straightforward:
- keep one shared Solana program ID
- keep one shared escrow engine
- do not force one account type to represent every escrow product

Recommended contract shape:
- `Trade` for marketplace flows
- `PrivateEscrow` for private link deals
- `ServiceEscrow` for service jobs

Shared internal logic should cover:
- custody
- release
- cancel
- dispute
- fee handling
- admin controls

---

## Current Contract Status

The current program is still marketplace-first, but it already contains the foundation of a shared escrow engine.

### ✅ Implemented At The Interface Level

- `Trade`, `GlobalState`, `FrozenUser`, and `AdminLog` account models exist
- marketplace trade creation and acceptance exist
- escrow vault authority model exists
- buyer payment submission flow exists
- seller confirmation and release flow exist
- trade cancellation and auto-cancel hooks exist
- dispute resolution and admin force-close surface exist
- frozen-user and global pause controls exist
- fee configuration exists
- event surface exists for trade lifecycle, moderation, and admin actions

### ✅ Hardening Already Done

Based on the current audit notes and interface updates, the following have already been addressed in the program direction:
- admin authority has been moved toward `global_state.admin`
- vault authority is now part of the contract account model
- frozen-user checks are wired into the instruction interfaces
- input validation has been tightened
- relist behavior has been tightened
- tests were updated to the current instruction/account shape

### 🚧 Still Not Clean Enough

- deprecated `mark_completed` still exists and should not remain an easy release path
- fee config validation still needs to be bounded cleanly
- dispute and admin-path test coverage still looks incomplete
- this repo snapshot does not include the Rust `programs/p2p/src` tree, so the roadmap can only reflect the public interface and audit notes, not a full source audit

### 📌 Interpretation

- the repo already contains the seed of a shared escrow engine
- the current `Trade` model is still too marketplace-specific to become the universal escrow record
- that is acceptable for now
- `Trade` should remain the marketplace account, not the base type for every future escrow mode

---

## Architectural Principles

### 1. One Program, Multiple Account Models

Keep one program ID as the escrow engine.
Add separate account types per product mode instead of overloading `Trade`.

Why:
- marketplace, private links, and service jobs do not have the same lifecycle details
- forcing a single struct will create brittle branching logic and audit risk

### 2. Shared Lifecycle, Mode-Specific Rules

High-level lifecycle should stay consistent:
- `create`
- `fund`
- `confirm`
- `release`
- `dispute`

Internal status design should be more explicit:
- `created`
- `awaiting_funding`
- `funded`
- `in_progress`
- `delivered`
- `completed`
- `cancelled`
- `disputed`
- `resolved`
- `expired`

Not every mode must expose every label to the user, but the engine should follow a normalized model.

### 3. Marketplace Stays Supported

Do not break the current marketplace flow while adding new escrow modes.

That means:
- keep `Trade` stable
- avoid unnecessary account migrations
- preserve existing event semantics where possible
- treat deprecated instructions carefully

### 4. Security Work Comes Before New Surface Area

Before adding new escrow products, the marketplace contract path needs to be hardened.

That includes:
- deprecated instruction cleanup
- admin authorization consistency
- role enforcement by trade type
- fee validation
- account constraint tightening
- better test coverage around failures and disputes

---

## Contract Phases

## Phase 1 - Marketplace Hardening

Objective:
stabilize the existing marketplace escrow engine before extending it.

### ✅ Already Implemented Or Largely Addressed

- `seller_confirm_received` exists as the intended canonical release path
- admin authority has been aligned around `global_state.admin`
- vault authority is part of the account model
- frozen-user checks are represented in instruction interfaces
- trade-type role logic was called out and moved toward correction
- relist-state cleanup was addressed
- tests were updated to the current interface shape

### 🚧 Remaining Priority Items

- remove or hard-gate deprecated `mark_completed`
- validate fee config bounds cleanly
- confirm every transfer path enforces vault authority ownership exactly as intended
- make frozen-user enforcement strict and unambiguous in all paths
- finish failure-path coverage for:
  - disputes
  - force close
  - freeze/unfreeze
  - paused system behavior
  - invalid role attempts
  - fee-on-release behavior

### 📌 Phase 1 Outcome

Phase 1 should end with one clear rule:
- escrow release must only happen through the intended, auditable lifecycle
- not through legacy shortcuts or loosely guarded instructions

Definition of done:
- marketplace flow is internally coherent
- legacy shortcuts cannot bypass the intended escrow lifecycle
- failure-path tests are as strong as happy-path tests

## Phase 2 - Shared Escrow Foundation

Objective:
extract the common escrow engine concepts that future account types will reuse.

Likely shared components:
- common status model
- common custody helpers
- release distribution helpers
- fee calculation helpers
- dispute metadata model
- admin audit/event model
- timeout and expiry helpers

Design requirement:
- shared logic can live in internal modules
- public instruction surfaces should remain mode-specific

Definition of done:
- new escrow modes can reuse internals without inheriting marketplace assumptions

## Phase 3 - Private Link Contract Layer

Objective:
add true on-chain support for private escrow links.

New account type:
- `PrivateEscrow`

Expected instruction surface:
- `create_private_escrow`
- `fund_private_escrow`
- `confirm_private_escrow`
- `release_private_escrow`
- `cancel_private_escrow`
- `dispute_private_escrow`

Core requirements:
- support intent model:
  - `I'm Paying`
  - `I'm Receiving`
- derive who funds and who receives from escrow intent
- support proof metadata without turning the program into a document store
- support backend linkage by storing or mapping a stable escrow ID
- preserve admin dispute override path

Definition of done:
- private link backend records no longer simulate escrow state off-chain
- funding and release become contract-enforced

## Phase 4 - Service Escrow Contract Layer

Objective:
support service-based escrow without contorting marketplace logic.

New account type:
- `ServiceEscrow`

Expected instruction surface:
- `create_service_escrow`
- `fund_service_escrow`
- `mark_delivered`
- `accept_delivery`
- `release_service_escrow`
- `cancel_service_escrow`
- `dispute_service_escrow`

Additional design needs:
- milestone or delivery semantics may differ from marketplace settlement
- service completion should not reuse buyer/seller naming blindly
- release rules may need dual confirmation or delivery acceptance logic

Definition of done:
- service escrow works as a first-class contract flow, not a renamed marketplace trade

## Phase 5 - Unified Events, Indexing, and Fees

Objective:
make all escrow modes observable and operationally consistent.

Required work:
- standardize event naming across escrow modes
- ensure each escrow mode emits enough metadata for backend analytics
- define fee behavior consistently across marketplace, private, and service flows
- clarify which fields are mandatory for analytics vs optional for UX
- keep IDL predictable for backend consumers

Suggested event families:
- created
- funded
- payment_submitted
- confirmed
- released
- cancelled
- disputed
- resolved
- force_closed

Definition of done:
- backend analytics and admin tooling can consume all escrow modes without ad hoc parsing

## Phase 6 - Versioning and Migration Safety

Objective:
grow the contract without breaking existing marketplace integrations.

Required work:
- maintain backward compatibility where reasonable
- deprecate old instructions explicitly
- document migration strategy for clients and backend services
- define when a new account type is additive vs when a breaking change is unavoidable
- keep test fixtures for old marketplace flows while adding new escrow modes

Definition of done:
- repo can evolve into a multi-mode escrow engine without unstable contract integration

---

## Recommended Build Order

1. Finish marketplace hardening
2. Extract shared escrow internals
3. Add `PrivateEscrow`
4. Add `ServiceEscrow`
5. Normalize events, fees, and analytics hooks
6. Document versioning and migration policy

This order is important.
Adding new product modes before Phase 1 is complete will compound audit and support risk.

---

## What I Think

The direction is correct.

The strongest part of the app roadmap is this decision:
- Solia should be an escrow engine with multiple product surfaces, not just a P2P market

The strongest part of the contract direction is this decision:
- one shared program
- separate account types per escrow mode

That is the right boundary.

The main thing to avoid is trying to make `Trade` become:
- marketplace trade
- private payment request
- OTC link
- service delivery contract

That would make the contract harder to reason about, harder to audit, and harder to test.

The right approach is:
- keep shared escrow mechanics centralized
- keep escrow account models specialized

---

## Non-Goals For This Repo

Do not treat these as contract deliverables here:
- UI renaming tasks
- navigation changes
- landing page branding
- notification UX
- admin panel layout work
- mobile parity issues unrelated to contract behavior

They may depend on the contract roadmap, but they should not drive this repo's structure.

---

## Immediate Next Step

If we continue from this roadmap, the next useful work in this repo is:
- define the exact Phase 1 hardening checklist against the current program
- map existing instructions into reusable shared escrow primitives
- design the `PrivateEscrow` account and instruction set before writing code
