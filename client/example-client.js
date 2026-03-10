const fs = require("fs");
const os = require("os");
const path = require("path");
const anchor = require("@coral-xyz/anchor");
const {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} = require("@solana/spl-token");

function loadIdl() {
  return JSON.parse(
    fs.readFileSync(path.join(__dirname, "..", "target", "idl", "p2p.json"), "utf8")
  );
}

function loadKeypair() {
  const keypairPath =
    process.env.ANCHOR_WALLET ||
    path.join(os.homedir(), ".config", "solana", "id.json");
  const secret = JSON.parse(fs.readFileSync(keypairPath, "utf8"));
  return anchor.web3.Keypair.fromSecretKey(Uint8Array.from(secret));
}

function clusterUrl() {
  return process.env.SOLANA_RPC_URL || "https://api.devnet.solana.com";
}

function getProvider() {
  const connection = new anchor.web3.Connection(clusterUrl(), "confirmed");
  const wallet = new anchor.Wallet(loadKeypair());
  return new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
}

function getProgram(provider) {
  const idl = loadIdl();
  return new anchor.Program(idl, provider);
}

function pdas(program, user) {
  const [globalState] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("global_state")],
    program.programId
  );
  const [vaultAuthority] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault-authority")],
    program.programId
  );
  const [frozenStatus] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("frozen_user"), user.toBuffer()],
    program.programId
  );
  return { globalState, vaultAuthority, frozenStatus };
}

function usage() {
  console.log(`
Usage:
  node client/example-client.js init-global-state
  node client/example-client.js create-sell-trade <mint> <amount> <paymentToken> <paymentWallet> <expectedPaymentAmount>
  node client/example-client.js accept-trade <trade> <counterpartyTokenAccount>
  node client/example-client.js complete-trade <trade> <initiatorTokenAccount> <counterpartyTokenAccount>
  node client/example-client.js show-trade <trade>

Environment:
  ANCHOR_WALLET   Path to the signing keypair json
  SOLANA_RPC_URL  RPC URL, defaults to devnet
`);
}

async function initGlobalState(program) {
  const provider = program.provider;
  const { globalState } = pdas(program, provider.wallet.publicKey);

  const signature = await program.methods
    .initializeGlobalState(provider.wallet.publicKey)
    .accounts({
      globalState,
      admin: provider.wallet.publicKey,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  console.log("Global state:", globalState.toBase58());
  console.log("Signature:", signature);
}

async function createSellTrade(
  program,
  mintArg,
  amountArg,
  paymentToken,
  paymentWallet,
  expectedPaymentArg
) {
  const provider = program.provider;
  const mint = new anchor.web3.PublicKey(mintArg);
  const amount = new anchor.BN(amountArg);
  const expectedPaymentAmount = new anchor.BN(expectedPaymentArg);
  const trade = anchor.web3.Keypair.generate();
  const { globalState, vaultAuthority, frozenStatus } = pdas(
    program,
    provider.wallet.publicKey
  );
  const initiatorTokenAccount = getAssociatedTokenAddressSync(
    mint,
    provider.wallet.publicKey
  );
  const vaultTokenAccount = getAssociatedTokenAddressSync(
    mint,
    vaultAuthority,
    true
  );

  const signature = await program.methods
    .createTrade(
      amount,
      { sell: {} },
      0,
      paymentToken,
      paymentWallet,
      expectedPaymentAmount
    )
    .accounts({
      trade: trade.publicKey,
      initiator: provider.wallet.publicKey,
      mint,
      initiatorTokenAccount,
      vaultTokenAccount,
      globalState,
      frozenStatus,
      vaultAuthority,
      systemProgram: anchor.web3.SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .signers([trade])
    .rpc();

  console.log("Trade:", trade.publicKey.toBase58());
  console.log("Signature:", signature);
}

async function acceptTrade(program, tradeArg, counterpartyTokenAccountArg) {
  const provider = program.provider;
  const trade = new anchor.web3.PublicKey(tradeArg);
  const counterpartyTokenAccount = new anchor.web3.PublicKey(counterpartyTokenAccountArg);
  const tradeAccount = await program.account.trade.fetch(trade);
  const { globalState, vaultAuthority, frozenStatus } = pdas(
    program,
    provider.wallet.publicKey
  );
  const vaultTokenAccount = getAssociatedTokenAddressSync(
    tradeAccount.mint,
    vaultAuthority,
    true
  );

  const signature = await program.methods
    .acceptTrade()
    .accounts({
      trade,
      counterparty: provider.wallet.publicKey,
      counterpartyTokenAccount,
      vaultTokenAccount,
      vaultAuthority,
      globalState,
      frozenStatus,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: anchor.web3.SystemProgram.programId,
      rent: anchor.web3.SYSVAR_RENT_PUBKEY,
    })
    .rpc();

  console.log("Signature:", signature);
}

async function completeTrade(program, tradeArg, initiatorTokenArg, counterpartyTokenArg) {
  const provider = program.provider;
  const trade = new anchor.web3.PublicKey(tradeArg);
  const initiatorTokenAccount = new anchor.web3.PublicKey(initiatorTokenArg);
  const counterpartyTokenAccount = new anchor.web3.PublicKey(counterpartyTokenArg);
  const tradeAccount = await program.account.trade.fetch(trade);
  const { vaultAuthority } = pdas(program, provider.wallet.publicKey);
  const vaultTokenAccount = getAssociatedTokenAddressSync(
    tradeAccount.mint,
    vaultAuthority,
    true
  );

  const signature = await program.methods
    .markCompleted()
    .accounts({
      trade,
      user: provider.wallet.publicKey,
      initiatorTokenAccount,
      counterpartyTokenAccount,
      vaultTokenAccount,
      vaultAuthority,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .rpc();

  console.log("Signature:", signature);
}

async function showTrade(program, tradeArg) {
  const trade = new anchor.web3.PublicKey(tradeArg);
  const tradeAccount = await program.account.trade.fetch(trade);
  console.log(JSON.stringify(tradeAccount, null, 2));
}

async function main() {
  const [command, ...args] = process.argv.slice(2);
  if (!command) {
    usage();
    process.exit(1);
  }

  const provider = getProvider();
  anchor.setProvider(provider);
  const program = getProgram(provider);

  switch (command) {
    case "init-global-state":
      return initGlobalState(program);
    case "create-sell-trade":
      if (args.length !== 5) return usage();
      return createSellTrade(program, ...args);
    case "accept-trade":
      if (args.length !== 2) return usage();
      return acceptTrade(program, ...args);
    case "complete-trade":
      if (args.length !== 3) return usage();
      return completeTrade(program, ...args);
    case "show-trade":
      if (args.length !== 1) return usage();
      return showTrade(program, ...args);
    default:
      usage();
      process.exit(1);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
