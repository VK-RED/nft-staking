use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo}, clock::Clock, entrypoint::ProgramResult, msg, program::invoke_signed, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, sysvar::Sysvar
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token::{instruction::{close_account, mint_to_checked, transfer}, state::{Account, Mint}};

use crate::{errors::NftStakingError, state::{Stake, StakeDetails}};

pub fn unstake(program_id: &Pubkey, accounts:&[AccountInfo]) -> ProgramResult{

    let iter = &mut accounts.iter();

    let user_account = next_account_info(iter)?;

    if !user_account.is_signer || !user_account.is_writable {
        msg!("User Account is Not Signer or Writable");
        return Err(ProgramError::MissingRequiredSignature);
    }
    
    let user_nft_token_account = next_account_info(iter)?;
    if !user_nft_token_account.is_writable {
        msg!("User NFT Token Account is Not Writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let user_reward_token_account = next_account_info(iter)?;
    if !user_reward_token_account.is_writable {
        msg!("User Reward Token Account is Not Writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let stake_details_account = next_account_info(iter)?;

    if stake_details_account.owner != program_id {
        msg!("Stake Details Account is not owned by the program");
        return Err(ProgramError::InvalidAccountData);
    }

    let stake_account = next_account_info(iter)?;
    if !stake_account.is_writable {
        msg!("Stake Account is Not Writable");
        return Err(ProgramError::InvalidAccountData);
    }

    if stake_account.owner != program_id {
        msg!("Stake Account is not owned by the program");
        return Err(ProgramError::InvalidAccountData);
    }

    let stake_nft_token_account = next_account_info(iter)?;
    if !stake_nft_token_account.is_writable {
        msg!("Stake NFT Token Account is Not Writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let nft_mint_account = next_account_info(iter)?;

    let reward_token_mint_account = next_account_info(iter)?;
    if !reward_token_mint_account.is_writable {
        msg!("Reward Token Mint Account is Not Writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let token_program = next_account_info(iter)?;

    if stake_account.owner != program_id {
        msg!("Stake Account not owned by the program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let seeds = [
        b"stake", 
        stake_details_account.key.as_ref(), 
        nft_mint_account.key.as_ref(), 
        user_account.key.as_ref()
    ];

    let (stake_account_key, stake_bump) = Pubkey::find_program_address(&seeds, program_id);

    if stake_account_key != *stake_account.key{
        msg!("Invalid Stake Account, expected : {}", stake_account_key);
        return Err(ProgramError::InvalidAccountData);
    }

    if stake_account.data.borrow().len() == 0 {
        msg!("Stake Account is Not Initialized");
        return Err(NftStakingError::AccountNotInitialized.into());
    }

    let stake_nft_token_key = get_associated_token_address_with_program_id(
        stake_account.key, 
        nft_mint_account.key, 
        token_program.key
    );

    if stake_nft_token_key != *stake_nft_token_account.key {
        msg!("Invalid Stake NFT Token Account , expected : {}", stake_nft_token_key);
        return Err(ProgramError::InvalidAccountData);
    }

    let stake_nft_token_account_data = Account::unpack(&stake_nft_token_account.data.borrow())?;

    if stake_nft_token_account_data.amount == 0 {
        msg!("No NFT Found in stake_nft_token_account : {}", stake_nft_token_account.key);
        return Err(NftStakingError::NoNFTFound.into())
    }

    let stake_data = Stake::try_from_slice(&stake_account.data.borrow())?;
    let stake_details_data = StakeDetails::try_from_slice(&stake_details_account.data.borrow())?;
    

    let transfer_ix = transfer(
        token_program.key, 
        stake_nft_token_account.key, 
        user_nft_token_account.key, 
        stake_account.key, 
        &[stake_account.key], 
        stake_nft_token_account_data.amount
    )?;

    msg!("Transferring NFT back to the user");

    let signers_seeds = [
        b"stake",
        stake_details_account.key.as_ref(),
        stake_data.nft_mint.as_ref(),
        user_account.key.as_ref(),
        &[stake_bump]
    ];

    invoke_signed(
        &transfer_ix, 
        &[
            stake_account.clone(),
            stake_nft_token_account.clone(),
            user_nft_token_account.clone(),
            token_program.clone(),
        ], 
        &[&signers_seeds]
    )?;

    let now = Clock::get()?.unix_timestamp;

    let staked_duration = now - stake_data.staked_at;
    let reward_amount = (staked_duration * 1000) as u64; // considering 1000 tokens per second

    let reward_token_data = Mint::unpack(&reward_token_mint_account.data.borrow())?;

    msg!("Sending {} reward tokens to the account : {}", reward_amount, user_reward_token_account.key);

    let mint_ix = mint_to_checked(
        token_program.key, 
        &reward_token_mint_account.key, 
        user_reward_token_account.key, 
        stake_details_account.key, 
        &[stake_details_account.key], 
        reward_amount,
        reward_token_data.decimals
    )?;

    let stake_details_seeds = [
        b"stake_details",
        stake_details_data.creator.as_ref(),
        stake_details_data.collection_mint.as_ref(),
        &[stake_details_data.bump_seed]
    ];

    invoke_signed(
        &mint_ix, 
        &[
            stake_details_account.clone(),
            user_reward_token_account.clone(),
            reward_token_mint_account.clone(),
        ], 
        &[&stake_details_seeds]
    )?;

    msg!("Closing Stake NFT Account");

    let close_ix = close_account(
        token_program.key, 
        stake_nft_token_account.key, 
        user_account.key, 
        stake_account.key, 
        &[stake_account.key]
    )?;

    invoke_signed(
        &close_ix, 
        &[
            stake_account.clone(),
            stake_nft_token_account.clone(),
            user_account.clone(),
            token_program.clone(),
        ], 
        &[&signers_seeds]
    )?;
    

    let stake_account_lamports = stake_account.lamports();
    let user_account_lamports = user_account.lamports();

    let final_user_lamports = user_account_lamports.checked_add(stake_account_lamports);

    if let None = final_user_lamports {
        msg!("Lamports Addition Overlfow Error");
        return Err(NftStakingError::StakeAccountCloseError.into());
    }

    msg!("Transferring Lamports from Stake Account to User Account");
    let mut user_lamports = user_account.lamports.borrow_mut();
    **user_lamports = final_user_lamports.unwrap();

    msg!("Resetting Stake Account Lamports to 0");
    let mut stake_account_lamports = stake_account.lamports.borrow_mut();
    **stake_account_lamports = 0;

    msg!("Closing Stake Account");

    stake_account.assign(&solana_program::system_program::ID);
    stake_account.realloc(0, false)?;

    Ok(())
}