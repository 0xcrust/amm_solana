import * as anchor from '@project-serum/anchor';
import * as spl from '@solana/spl-token';

export const createTokenMint = async (connection: anchor.web3.Connection, mintAuthority: anchor.web3.Keypair, decimals: number)
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
      decimals
    );
    console.log(`Mint account created with address: ${mintAddress.toBase58()}`);
  
    return [mintAddress, mintAuthority];
}


export const airdrop = async (connection: anchor.web3.Connection, destinationWallet: anchor.web3.PublicKey, amount) => {
    const airdropSignature = await connection.requestAirdrop(destinationWallet, amount * anchor.web3.LAMPORTS_PER_SOL);

    const latestBlockHash = await connection.getLatestBlockhash();

    await connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: airdropSignature,
    });
    console.log(`Airdropped ${amount} sol to ${destinationWallet}!`);
}

export const createATA = async(connection: anchor.web3.Connection, account: anchor.web3.Keypair, mint: anchor.web3.PublicKey)
: Promise<anchor.web3.PublicKey> => {
    const wallet = await spl.createAssociatedTokenAccount(
        connection,
        account,
        mint,
        account.publicKey
    );

    console.log("Created Associated Token Account");
    return wallet;
}

export const mintTokensToWallet = async(connection: anchor.web3.Connection, wallet: anchor.web3.PublicKey, 
    amount: number, feePayer: anchor.web3.Keypair, mintAddress: anchor.web3.PublicKey, mintAuthority: anchor.web3.Keypair) => {
    let tx = await spl.mintToChecked(
        connection,
        feePayer,
        mintAddress,
        wallet,
        mintAuthority,
        amount * 1e0,
        0
    );

    console.log(`Minted ${amount} tokens to ${wallet}`);
}