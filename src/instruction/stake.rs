use borsh::{BorshDeserialize, BorshSerialize};
use mpl_token_metadata::accounts::Metadata;
use spl_token::state::Account;
use spl_associated_token_account::instruction::create_associated_token_account;
use solana_program::{account_info::{next_account_info, AccountInfo}, clock::Clock, entrypoint::ProgramResult, msg, program::{invoke, invoke_signed}, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, rent::Rent, system_instruction, sysvar::Sysvar};
use crate::{errors::NftStakingError, state::{Stake,StakeDetails}};


pub fn stake(
    program_id: &Pubkey,
    accounts: &[AccountInfo]
) -> ProgramResult{

    let iter = &mut accounts.iter();

    let user = next_account_info(iter)?;
    if !user.is_signer || !user.is_writable {
        msg!("User account is not signer or writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let nft_mint = next_account_info(iter)?;
    let nft_metadata_account = next_account_info(iter)?; // metadata account of the staking nft

    let user_token_account = next_account_info(iter)?; // nft token account of user

    if !user_token_account.is_writable {
        msg!("User Token Account is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let user_reward_token_account = next_account_info(iter)?; // reward token account for the user

    let stake_details_account = next_account_info(iter)?; 

    if stake_details_account.owner != program_id {
        msg!("Stake Details Account is not owned by the program");
        return Err(ProgramError::InvalidAccountData);
    }

    let stake_account = next_account_info(iter)?; // pda

    if !stake_account.is_writable {
        msg!("Stake account is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let stake_ata = next_account_info(iter)?; // ata of the stake account to store the user nft
    
    if !stake_ata.is_writable {
        msg!("Stake ATA is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let token_program = next_account_info(iter)?;
    let associated_token_program = next_account_info(iter)?;
    let system_program = next_account_info(iter)?;

    let onchain_data = nft_metadata_account.data.borrow_mut();
        
    let metadata = Metadata::safe_deserialize(&onchain_data)?;

    let nft_collection = metadata.collection.ok_or(NftStakingError::NoCollectionSet)?;

    let stake_details = StakeDetails::try_from_slice(&*stake_details_account.data.borrow())?;

    // validate the metadata account points to the nft mint
    if metadata.mint != *nft_mint.key {
        msg!("NFT mint key mismatch in NFT metadata Account");
        return Err(NftStakingError::InvalidMetadataAccount.into());
    }

    // validate the nft is part of the staking details collection
    if stake_details.collection_mint != nft_collection.key {
        msg!("NFT does not belong to the Stake Details collection");
        return Err(NftStakingError::CollectionMintMismatch.into())
    }

    // validate the nft is verified
    if !nft_collection.verified {
        msg!("NFT is not verified");
        return Err(NftStakingError::NftNotVerified.into());
    }

    let user_token_account_data = Account::unpack(&user_token_account.data.borrow())?;

    if user_token_account_data.amount == 0 {
        msg!("User have 0 NFT");
        return Err(NftStakingError::NftEmpty.into());
    }

    let seeds = [
        b"stake", 
        stake_details_account.key.as_ref(), 
        nft_mint.key.as_ref(), 
        user.key.as_ref()
    ];

    let (stake_key, stake_key_bump) = Pubkey::find_program_address(&seeds, program_id);

    if stake_key != *stake_account.key {
        msg!("Stake key mismatch");
        msg!("Expected stake key : {}, received : {}", stake_key, stake_account.key);    
        return Err(ProgramError::InvalidAccountData);
    }

    // initialize stake account

    let space : usize = Stake::LEN ;
    let lamports : u64 = Rent::get()?.minimum_balance(space);

    let stake_ix = system_instruction::create_account(
        user.key, 
        stake_account.key, 
        lamports, 
        space as u64, 
        program_id
    );

    invoke_signed(
        &stake_ix, 
        &[user.clone(), stake_account.clone(), system_program.clone()], 
        &[&[
            b"stake", 
            stake_details_account.key.as_ref(), 
            nft_mint.key.as_ref(), 
            user.key.as_ref(),
            &[stake_key_bump],
        ]]
    )?;

    msg!("Stake account Successfully initialized");

    let ata_seeds = [
        stake_account.key.as_ref(),
        token_program.key.as_ref(),
        nft_mint.key.as_ref()
    ];

    let (stake_ata_key, _stake_ata_bump) = Pubkey::find_program_address(&ata_seeds, associated_token_program.key);

    if stake_ata_key != *stake_ata.key {
        msg!("Stake ATA key mismatch");
        msg!("Expected Stake ATA key : {}, received : {}", stake_ata_key, stake_ata.key);    
        return Err(ProgramError::InvalidAccountData);
    }

    msg!("Expected Stake ATA key : {}, received : {}", stake_ata_key, stake_ata.key);    

    // initialize stake ata
    let stake_ata_ix = create_associated_token_account(
        user.key,
        stake_account.key,
        nft_mint.key,
        token_program.key
    );

    invoke_signed(
        &stake_ata_ix,
        &[
            user.clone(), 
            stake_ata.clone(), 
            stake_account.clone(), 
            nft_mint.clone(), 
            associated_token_program.clone(), 
            token_program.clone(), 
            system_program.clone()
        ],
        &[&[
            b"stake", 
            stake_details_account.key.as_ref(), 
            nft_mint.key.as_ref(), 
            user.key.as_ref(),
            &[stake_key_bump],
        ]]
    )?;


    msg!("Associated Token Account for the Stake Account has been created");

    // transfer nft from user ata to stake ata

    let transfer_ix = spl_token::instruction::transfer(
        token_program.key, 
        user_token_account.key, 
        stake_ata.key, 
        user.key, 
        &[user.key], 
        1 
    )?;

    invoke(
        &transfer_ix, 
        &[user.clone(), user_token_account.clone(), stake_ata.clone(), token_program.clone()]
    )?;
    

    msg!("Successfully Transferred nft from user to stake");

    let staked_at = Clock::get()?.unix_timestamp;

    let stake = Stake{
        nft_mint: *nft_mint.key,
        reward_mint: stake_details.reward_token_mint,
        reward_mint_ata: *user_reward_token_account.key,
        stake_details_key: *stake_details_account.key,
        staked_at,
    };

    stake.serialize(&mut *stake_account.data.borrow_mut())?;    

    msg!("Successfully written stake data to onchain");
    Ok(())

    
}