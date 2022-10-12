import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { assert } from "chai";
import { Torrent } from "../target/types/torrent";
import {
  airdrop,
  createTokenMint
} from "./utils";

describe("Torrent", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Torrent as Program<Torrent>;

  const authority = anchor.web3.Keypair.generate();
  const mintAuthority = anchor.web3.Keypair.generate();
  
  it("initializes torrent and pool!", async () => {
    // Airdrop sol to authority
    await airdrop(provider.connection, authority, 1);

    let [torrentPDA, torrentBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("torrent")), authority.publicKey
      .toBuffer()], program.programId
    );

    let [liquidityTokenMint, ltBump] = await anchor.web3.PublicKey.findProgramAddress([
      Buffer.from(anchor.utils.bytes.utf8.encode("token")), torrentPDA.toBuffer()
    ], program.programId);
    
    await program.methods
      .initializeTorrent(6)
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

    await airdrop(provider.connection, mintAuthority, 1);
    let [xTokenMint, xBump] = await createTokenMint(provider.connection, mintAuthority);
    let [yTokenMint, yBump] = await createTokenMint(provider.connection, mintAuthority);
});
