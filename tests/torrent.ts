import * as anchor from "@project-serum/anchor";
import * as spl from '@solana/spl-token';
import { Program, splitArgsAndCtx } from "@project-serum/anchor";
import { assert } from "chai";
import { Torrent } from "../target/types/torrent";
import {
  airdrop,
  createTokenMint,
  createATA,
  mintTokensToWallet,
  customGetTokenAccountBalance
} from "./utils";
import { TokenError } from "@solana/spl-token";

describe("Torrent", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Torrent as Program<Torrent>;

  const authority = anchor.web3.Keypair.generate();
  const mintAuthority = anchor.web3.Keypair.generate();

  let torrentPDA: anchor.web3.PublicKey;
  let liquidityTokenMint: anchor.web3.PublicKey;
  let xTokenMint: anchor.web3.PublicKey;
  let yTokenMint: anchor.web3.PublicKey;
  let xyPool: anchor.web3.PublicKey;
  let xVault: anchor.web3.PublicKey;
  let yVault: anchor.web3.PublicKey;

  let torrentBump: number;
  let ltBump: number;
  let xyPoolBump: number;
  let xVaultBump: number;
  let yVaultBump: number;
  
  it("initializes torrent and pool!", async () => {
    // Airdrop sol to authority
    await airdrop(provider.connection, authority.publicKey, 1);

    [torrentPDA, torrentBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("torrent")), authority.publicKey
      .toBuffer()], program.programId
    );

    [liquidityTokenMint, ltBump] = await anchor.web3.PublicKey.findProgramAddress([
      Buffer.from(anchor.utils.bytes.utf8.encode("token")), torrentPDA.toBuffer()
    ], program.programId);
    
    let ltDecimals = 0;
    await program.methods
      .initializeTorrent(ltDecimals)
      .accounts({
        authority: authority.publicKey,
        torrent: torrentPDA,
        liquidityToken: liquidityTokenMint
      })
      .signers([authority])
      .rpc();

    let torrentState = await program.account.torrent.fetch(torrentPDA);

    assert.ok(torrentState.authority.equals(authority.publicKey));
    assert.ok(torrentState.liquidityTokenMint.equals(liquidityTokenMint));
    assert.equal(torrentState.torrentLiquidity.toNumber(), 0);

    await airdrop(provider.connection, mintAuthority.publicKey, 1);
    xTokenMint = await createTokenMint(provider.connection, mintAuthority, 0);
    yTokenMint = await createTokenMint(provider.connection, mintAuthority, 0);

    // Create token accounts
    let authorityXWallet = await createATA(provider.connection, authority, xTokenMint);
    let authorityYWallet = await createATA(provider.connection, authority, yTokenMint);
    let authorityLtWallet = await createATA(provider.connection, authority, liquidityTokenMint);

    // Mint 10 xTokens and 8 yTokens
    let initialX = 10;
    let initialY = 8;
    let expectedMintAmount = (initialX + initialY) / 2;

    await mintTokensToWallet(provider.connection, authorityXWallet, initialX + 2, mintAuthority, 
      xTokenMint, mintAuthority);
    await mintTokensToWallet(provider.connection, authorityYWallet, initialY + 2, mintAuthority, 
      yTokenMint, mintAuthority);

    [xyPool, xyPoolBump] = await anchor.web3.PublicKey.findProgramAddress([
      torrentPDA.toBuffer(), xTokenMint.toBuffer(), yTokenMint.toBuffer()
    ], program.programId);

    [xVault, xVaultBump] = await anchor.web3.PublicKey.findProgramAddress([
      Buffer.from(anchor.utils.bytes.utf8.encode("x_vault")), xyPool.toBuffer()
    ], program.programId);

    [yVault, yVaultBump] = await anchor.web3.PublicKey.findProgramAddress([
      Buffer.from(anchor.utils.bytes.utf8.encode("y_vault")), xyPool.toBuffer()
    ], program.programId);

    try {
    await program.methods
      .initializePool(new anchor.BN(initialX), new anchor.BN(initialY))
      .accounts({
        torrent: torrentPDA,
        liquidityTokenMint: liquidityTokenMint,
        authority: authority.publicKey,
        mintX: xTokenMint,
        mintY: yTokenMint,
        authorityXWallet: authorityXWallet,
        authorityYWallet: authorityYWallet,
        authorityLiquidityTokenWallet: authorityLtWallet,
        pool: xyPool,
        xTokenVault: xVault,
        yTokenVault: yVault,
      })
      .signers([authority])
      .rpc();
    } catch(_err) {
      console.log(_err);
    }

    torrentState = await program.account.torrent.fetch(torrentPDA);
    let poolState = await program.account.pool.fetch(xyPool);
    let poolIndex = poolState.index;

    assert.ok(poolState.torrent.equals(torrentPDA));
    assert.ok(torrentState.pools[poolIndex].equals(xyPool));
    assert.equal(poolState.poolLiquidity.toNumber(), expectedMintAmount);
    assert.equal(torrentState.torrentLiquidity.toNumber(), expectedMintAmount);

    let xVaultBalance = await customGetTokenAccountBalance(provider.connection, xVault);
    let yVaultBalance = await customGetTokenAccountBalance(provider.connection, yVault);
    let authorityLtWalletBalance = await customGetTokenAccountBalance(provider.connection, authorityLtWallet);

    assert.equal(xVaultBalance, initialX);
    assert.equal(yVaultBalance, initialY);
    assert.equal(authorityLtWalletBalance, expectedMintAmount);
});

it ("Simulates adding liquidity", async () => {
  async function addLiquidity(amountX: number, amountY: number, liquidityProvider: anchor.web3.Keypair) {
    // airdrop sol to provider
    await airdrop(provider.connection, liquidityProvider.publicKey, 1);
    let xTokenATA = await createATA(provider.connection, liquidityProvider, xTokenMint);
    let yTokenATA = await createATA(provider.connection, liquidityProvider, yTokenMint);
    let liquidityTokenATA = await createATA(provider.connection, liquidityProvider, liquidityTokenMint);

    await mintTokensToWallet(provider.connection, xTokenATA, 20, liquidityProvider, xTokenMint, mintAuthority);
    await mintTokensToWallet(provider.connection, yTokenATA, 20, liquidityProvider, yTokenMint, mintAuthority);

    let xVaultBalance = await customGetTokenAccountBalance(provider.connection, xVault);
    let yVaultBalance = await customGetTokenAccountBalance(provider.connection, yVault);
    let expectedXAdded = amountX;
    let expectedYAdded = Math.trunc((yVaultBalance * amountX) / xVaultBalance);

    let poolState = await program.account.pool.fetch(xyPool);
    let torrentState = await program.account.torrent.fetch(torrentPDA);
    let poolLiquidity = poolState.poolLiquidity;
    let mintAmount = Math.trunc((amountX * poolLiquidity.toNumber()) / xVaultBalance);
    

    await program.methods
      .addLiquidity(new anchor.BN(amountX), new anchor.BN(amountY))
      .accounts({
        user: liquidityProvider.publicKey,
        torrent: torrentPDA,
        pool: xyPool,
        xTokenVault: xVault,
        yTokenVault: yVault,
        liquidityTokenMint: liquidityTokenMint,
        userXWallet: xTokenATA,
        userYWallet: yTokenATA,
        userLiquidityTokenWallet: liquidityTokenATA,
      })
      .signers([liquidityProvider]);

    let newXVaultBalance = await customGetTokenAccountBalance(provider.connection, xVault);
    let newYVaultBalance = await customGetTokenAccountBalance(provider.connection, yVault);
    let newliquidityBalance = await customGetTokenAccountBalance(provider.connection, liquidityTokenATA);
    
    let expectedNewXBalance = xVaultBalance + expectedXAdded;
    let expectedNewYBalance = yVaultBalance + expectedYAdded;
    
    assert.equal(newXVaultBalance, expectedNewXBalance);
    assert.equal(newYVaultBalance, expectedNewYBalance);
    assert.equal(newliquidityBalance, mintAmount);

    let newPoolState = await program.account.pool.fetch(xyPool);
    let newTorrentState = await program.account.torrent.fetch(torrentPDA);
    assert.equal(poolState.poolLiquidity.toNumber() + mintAmount, newPoolState.poolLiquidity.toNumber());
    assert.equal(torrentState.torrentLiquidity.toNumber() + mintAmount, newTorrentState.torrentLiquidity.toNumber());
  }

});

})