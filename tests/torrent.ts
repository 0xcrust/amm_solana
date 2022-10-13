import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { assert } from "chai";
import { Torrent } from "../target/types/torrent";
import {
  airdrop,
  createTokenMint,
  createATA,
  mintTokensToWallet,
} from "./utils";

describe("Torrent", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Torrent as Program<Torrent>;

  const authority = anchor.web3.Keypair.generate();
  const mintAuthority = anchor.web3.Keypair.generate();
  
  it("initializes torrent and pool!", async () => {
    // Airdrop sol to authority
    await airdrop(provider.connection, authority.publicKey, 1);

    let [torrentPDA, _torrentBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("torrent")), authority.publicKey
      .toBuffer()], program.programId
    );

    let [liquidityTokenMint, _ltBump] = await anchor.web3.PublicKey.findProgramAddress([
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
    let [xTokenMint, _xBump] = await createTokenMint(provider.connection, mintAuthority, 0);
    let [yTokenMint, _yBump] = await createTokenMint(provider.connection, mintAuthority, 0);

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

    let [poolPDA, _poolBump] = await anchor.web3.PublicKey.findProgramAddress([
      torrentPDA.toBuffer(), xTokenMint.toBuffer(), yTokenMint.toBuffer()
    ], program.programId);

    let [xVaultPDA, _xVaultBump] = await anchor.web3.PublicKey.findProgramAddress([
      Buffer.from(anchor.utils.bytes.utf8.encode("x_vault")), poolPDA.toBuffer()
    ], program.programId);

    let [yVaultPDA, _yVaultBump] = await anchor.web3.PublicKey.findProgramAddress([
      Buffer.from(anchor.utils.bytes.utf8.encode("y_vault")), poolPDA.toBuffer()
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
        pool: poolPDA,
        xTokenVault: xVaultPDA,
        yTokenVault: yVaultPDA,
      })
      .signers([authority])
      .rpc();
    } catch(_err) {
      console.log(_err);
    }

    torrentState = await program.account.torrent.fetch(torrentPDA);
    let poolState = await program.account.pool.fetch(poolPDA);
    let poolIndex = poolState.index;

    assert.ok(poolState.torrent.equals(torrentPDA));
    assert.ok(torrentState.pools[poolIndex].equals(poolPDA));
    assert.equal(poolState.poolLiquidity.toNumber(), expectedMintAmount);
    assert.equal(torrentState.torrentLiquidity.toNumber(), expectedMintAmount);

    let xVaultState = await provider.connection.getTokenAccountBalance(xVaultPDA);
    let yVaultState = await provider.connection.getTokenAccountBalance(yVaultPDA);
    let authorityLtWalletState = await provider.connection.getTokenAccountBalance(authorityLtWallet);

    assert.equal(xVaultState.value.uiAmount, initialX);
    assert.equal(yVaultState.value.uiAmount, initialY);
    assert.equal(authorityLtWalletState.value.uiAmount, expectedMintAmount);

});

})