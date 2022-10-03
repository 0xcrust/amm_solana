import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Torrent } from "../target/types/torrent";

describe("H20", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.H20 as Program<Torrent>;

  it("Is initialized!", async () => {
    // Add your test here.
    //const tx = await program.methods.initialize().rpc();
    //console.log("Your transaction signature", tx);
  });
});
