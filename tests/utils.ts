import * as anchor from '@project-serum/anchor';
import * as spl from '@solana/spl-token';

export const createTokenMint = async (connection: anchor.web3.Connection, mintAuthority: anchor.web3.Keypair)
: Promise<[anchor.web3.PublicKey, anchor.web3.Keypair]>  => {

    const airdropSignature3 = await connection.requestAirdrop(
        mintAuthority.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);

    const latestBlockHash3 = await connection.getLatestBlockhash();
    const mintAirdropTx = await connection.confirmTransaction({
      blockhash: latestBlockHash3.blockhash,
      lastValidBlockHeight: latestBlockHash3.lastValidBlockHeight,
      signature: airdropSignature3,
    });
  
    const mintAuthorityBalance = await connection.getBalance(mintAuthority.publicKey);
  
    let mintAddress = await spl.createMint(
      connection,
      mintAuthority,
      mintAuthority.publicKey,
      null,
      0
    );
    console.log(`Mint account created with address: ${mintAddress.toBase58()}`);
  
    return [mintAddress, mintAuthority];
}


export const airdrop = async (connection, destinationWallet: anchor.web3.Keypair, amount) => {
    const airdropSignature = await connection.requestAirdrop(destinationWallet
        .publicKey, amount * anchor.web3.LAMPORTS_PER_SOL);

    const latestBlockHash = await connection.getLatestBlockhash();

    await connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: airdropSignature,
    });
    console.log(`Airdropped ${amount} sol to ${destinationWallet.publicKey}!`);
}
