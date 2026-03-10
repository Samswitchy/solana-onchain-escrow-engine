# P2P Program Deployment Guide - Hello Solia Implementation

## Overview
This document details the complete process used to deploy the "Hello Solia" function in our P2P Anchor program to Solana Devnet.

**Submission Repo**: `https://github.com/SoliaNetwork/solia-p2p-escrow`  
**Smart Contract Repo**: `https://github.com/Solia-Network/solia_contracts.git`  
**App Repo**: `https://github.com/SoliaNetwork/solia_app.git`  

**Program ID**: `J9GcXnuwFQZqpA7rSXSt44Dt4zhtyZ1RQPZdSYfXWkpt`  
**Admin Wallet**: `38pEhDFkYMbZ9zxwthvku8bvyY2oFULVjZhzNhsw9NRR`
**Network**: Solana Devnet  
**Deployment Date**: October 16, 2025  
**Developer**: Abhishek Singh 

---

## 1. Anchor Version and Setup

### Version Information
- **Anchor CLI**: 0.32.1
- **Anchor Lang**: 0.31.1 (downgraded from 0.32.1 due to compatibility issues)
- **Rust**: 1.9.0
- **Solana CLI**: 2.3.0

### Why These Versions?
Initial setup used Anchor CLI 0.32.1, but anchor-lang was downgraded to 0.31.1 to resolve:
- proc-macro2 compatibility issues with `local_file()` method
- anchor-syn compilation errors
- regex_automata stack overflow errors

### Setup Commands Used
Configure Solana for devnet
- solana config set --url devnet

Get devnet SOL tokens
- solana airdrop 2
- solana balance

## 2. Project Structure
```bash
solia_contracts/
├── programs/
│ ├── escrow/ # Existing program (inactive)
│ └── p2p/ # Target program for deployment
│ ├── Cargo.toml # Updated dependencies and features
│ └── src/
│ └── lib.rs # Modified message to "Hello solia!"
├── target/deploy/ # Generated build artifacts
│ ├── p2p.so # Compiled program binary
│ └── p2p-keypair.json # Program keypair
├── Guideme.md # This documentation
├── Anchor.toml # Updated with deployed program IDs
└── Cargo.toml # Workspace configuration
```
### Important Changes Made
declare_id!("J9GcXnuwFQZqpA7rSXSt44Dt4zhtyZ1RQPZdSYfXWkpt");  // programID is changed
pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    msg!("Hello solia!");  // ← Key change: Updated message
    Ok(())
}


#### Anchor.toml
[programs.devnet]
p2p = "J9GcXnuwFQZqpA7rSXSt44Dt4zhtyZ1RQPZdSYfXWkpt" # ← Updated with new program ID

## 3. Deployment Steps
- Step 1: Environment Setup
- Step 2: Resolve Build Issues
   Remove problematic .DS_Store files (macOS compatibility issue)

   find . -name ".DS_Store" -delete

   Fix proc-macro2 compatibility

   cargo update -p proc-macro2 --precise 1.0.94
   
- Step 3: Build the Program
  Clean previous builds

    anchor clean

    cargo clean

    Build all programs

    anchor build

- Step 4: Deploy to Devnet


**Deployment Output:**
Program Id: J9GcXnuwFQZqpA7rSXSt44Dt4zhtyZ1RQPZdSYfXWkpt

Signature: (pending new deployment)

## 4. Calling the Function
### solana program show J9GcXnuwFQZqpA7rSXSt44Dt4zhtyZ1RQPZdSYfXWkpt
Output:  
Program Id: J9GcXnuwFQZqpA7rSXSt44Dt4zhtyZ1RQPZdSYfXWkpt
Owner: BPFLoaderUpgradeab1e11111111111111111111111
ProgramData Address: (pending new deployment)
Authority: (pending new deployment)
Last Deployed In Slot: (pending new deployment)
Data Length: (pending new deployment)
Balance: (pending new deployment)

### Creating Test to Call Function
- Created minimal test file

## 5. Key Learning Points

1. **Version Compatibility**: Anchor ecosystem has version dependencies that must be carefully managed
2. **Cross-Platform Issues**: macOS .DS_Store files can break Anchor builds on Linux systems
3. **Dependency Management**: proc-macro2 versions can cause stack overflow issues in newer releases
4. **Deployment Process**: Solana program deployment is separate from program function calls
5. **Configuration Management**: Program IDs must be synchronized across multiple configuration files

---

## 6. Future Recommendations

1. **Pin Exact Versions**: Use exact versions in Cargo.toml to avoid compatibility issues
2. **Automated Testing**: Implement CI/CD pipeline to catch version conflicts early  
3. **Documentation**: Maintain version compatibility matrix for the team
4. **Testing Framework**: Create comprehensive test suite for all program functions
5. **Monitoring**: Set up program monitoring on devnet/mainnet deployments

---
