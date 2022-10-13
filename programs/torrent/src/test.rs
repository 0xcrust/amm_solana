#[cfg(test)]
mod tests {
    use anchor_spl::token::{spl_token::state::Mint, ID as TOKEN_PROGRAM_ID};

    use anchor_client::{
        anchor_lang::{
            solana_program::{native_token::LAMPORTS_PER_SOL, sysvar::SysvarId},
            system_program,
        },
        solana_client::rpc_client::RpcClient,
        solana_sdk::{
            commitment_config::CommitmentConfig,
            instruction::Instruction,
            program_pack::Pack,
            pubkey::Pubkey,
            signature::{read_keypair_file, Keypair, Signer},
            system_transaction,
            transaction::Transaction,
        },
        Client, Cluster, Program,
    };

    use anyhow::Result;
    use rand::rngs::OsRng;
    use std::rc::Rc;

    use torrent;

    #[test]
    fn test_torrent_initialization() {
        let mut output = [0xFF; 32];
        bs58::decode("HzL5F7ePCv4bftNfSnMeGWtAYafJgGe5imiJjrD1Gd8n")
            .into(&mut output)
            .unwrap();
        let program_id: Pubkey = Pubkey::new_from_array(output);

        let dev_key: Keypair = read_keypair_file(&*shellexpand::tilde("../../provider.json"))
            .expect("Failed reading provider keypair");

        let url: Cluster = Cluster::Devnet;
        let client: Client =
            Client::new_with_options(url, Rc::new(dev_key), CommitmentConfig::processed());
        let program: Program = client.program(program_id);
        let rpc_client: RpcClient = program.rpc();

        let authority = Rc::new(Keypair::generate(&mut OsRng));
        let source: Keypair = read_keypair_file(&*shellexpand::tilde("../../provider.json"))
            .expect("Failed reading provider keypair");
        _ = fund_user(&rpc_client, &source, &authority.pubkey(), 1);
        
        let authority_pubkey = authority.pubkey();
        let torrent_seeds = &[b"torrent".as_ref(), authority_pubkey.as_ref()];
        let (torrent_pda, _) = Pubkey::find_program_address(torrent_seeds, &program_id);
        let liquidity_mint_seeds = &[b"token".as_ref(), torrent_pda.as_ref()];
        let (liquidity_mint, _) = Pubkey::find_program_address(liquidity_mint_seeds, &program_id);

        match program
            .request()
            .accounts(torrent::accounts::InitializeTorrent {
                authority: authority_pubkey,
                torrent: torrent_pda,
                liquidity_token: liquidity_mint,
                system_program: system_program::ID,
                token_program: TOKEN_PROGRAM_ID,
                rent: anchor_client::solana_sdk::rent::Rent::id(),
            })
            .args(torrent::instruction::InitializeTorrent { _decimals: 0 })
            .signer(&*authority)
            .payer(authority.clone())
            .send()
        {
            Ok(signature) => println!("Torrent initialized. Tx: {signature}"),
            Err(e) => panic!("{e:#?}"),
        };

        let torrent_state: torrent::Torrent = program.account(torrent_pda).unwrap();

        assert_eq!(torrent_state.authority, authority_pubkey);
        assert_eq!(torrent_state.liquidity_token_mint, liquidity_mint);
        assert_eq!(torrent_state.torrent_liquidity, 0);
    }

    #[allow(dead_code)]
    fn create_token_mint(
        mint_authority: &Keypair,
        rpc_client: &RpcClient,
        decimals: u8,
    ) -> Result<(Pubkey, String), anchor_client::solana_client::client_error::ClientError> {
        let mint = Keypair::generate(&mut OsRng);

        let pay_rent_and_create_account_ix: Instruction =
            anchor_client::solana_sdk::system_instruction::create_account(
                &mint_authority.pubkey(),
                &mint.pubkey(),
                rpc_client.get_minimum_balance_for_rent_exemption(Mint::LEN)?,
                Mint::LEN as u64,
                &TOKEN_PROGRAM_ID,
            );
        let initialize_mint_account_ix: Instruction =
            anchor_spl::token::spl_token::instruction::initialize_mint(
                &TOKEN_PROGRAM_ID,
                &mint.pubkey(),
                &mint_authority.pubkey(),
                None,
                decimals,
            )
            .expect("Failed to create initialize mint instruction");
        let spl_mint_tx = Transaction::new_signed_with_payer(
            &[pay_rent_and_create_account_ix, initialize_mint_account_ix],
            Some(&mint_authority.pubkey()),
            &[mint_authority, &mint],
            rpc_client
                .get_latest_blockhash()
                .expect("failed to get latest blockhash"),
        );

        // Send and confirm transaction. Get signature.
        let signature = rpc_client.send_and_confirm_transaction(&spl_mint_tx);
        let signature = signature.map(|s| s.to_string()).unwrap();
        Ok((mint.pubkey(), signature))
    }

    fn fund_user(
        rpc_client: &RpcClient,
        from: &Keypair,
        destination_wallet: &Pubkey,
        amount: u64,
    ) -> Result<()> {
        let fund_user_tx: Transaction = system_transaction::transfer(
            from,
            destination_wallet,
            amount * LAMPORTS_PER_SOL,
            rpc_client
                .get_latest_blockhash()
                .expect("failed to get latest blockhash"),
        );
        println!(
            "Wallet funded. Tx signature: {}",
            rpc_client
                .send_and_confirm_transaction(&fund_user_tx)
                .expect("Failed funding user")
        );
        Ok(())
    }

    #[allow(dead_code)]
    fn create_ata(rpc_client: &RpcClient, user: &Keypair, mint: &Pubkey) -> Result<Pubkey> {
        let user_ata: Pubkey =
            spl_associated_token_account::get_associated_token_address(&user.pubkey(), &mint);
        let spl_create_account_ix: Instruction =
            spl_associated_token_account::instruction::create_associated_token_account(
                &user.pubkey(),
                &user.pubkey(),
                &mint,
            );
        let create_spl_account_tx: Transaction = Transaction::new_signed_with_payer(
            &[spl_create_account_ix],
            Some(&user.pubkey()),
            &[user],
            rpc_client
                .get_latest_blockhash()
                .expect("failed getting latest blockhash"),
        );
        println!(
            "ATA created. Tx signatrue: {}",
            rpc_client
                .send_and_confirm_transaction(&create_spl_account_tx)
                .expect("failed creating token account")
        );

        Ok(user_ata)
    }

    #[allow(dead_code)]
    fn mint_tokens_to_wallet(
        rpc_client: &RpcClient,
        wallet: &Pubkey,
        mint: &Pubkey,
        mint_authority: &Keypair,
        amount: u64,
    ) -> Result<()> {
        let mint_ix: Instruction = anchor_spl::token::spl_token::instruction::mint_to(
            &TOKEN_PROGRAM_ID,
            &mint,
            &wallet,
            &mint_authority.pubkey(),
            &[&mint_authority.pubkey()],
            amount,
        )
        .expect("unable to mint tokens");

        let mint_tx: Transaction = Transaction::new_signed_with_payer(
            &[mint_ix],
            Some(&mint_authority.pubkey()),
            &[mint_authority],
            rpc_client
                .get_latest_blockhash()
                .expect("failed getting latest blockhash"),
        );

        println!(
            "Tokens minted. Tx signature: {}",
            rpc_client
                .send_and_confirm_transaction(&mint_tx)
                .expect("failed minting tokens to wallet")
        );

        Ok(())
    }
}
