use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{account_info::{next_account_info, AccountInfo}, clock::Clock, entrypoint::ProgramResult, msg, program::invoke_signed, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, sysvar::Sysvar};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token::{instruction::mint_to_checked, state::Mint};

use crate::{errors::NftStakingError, state::{Stake, StakeDetails}};

pub fn claim_rewards(program_id: &Pubkey, accounts_info:&[AccountInfo]) -> ProgramResult {

    let iter = &mut accounts_info.iter();

    let user_account = next_account_info(iter)?;

    if !user_account.is_signer {
        msg!("User account is not a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let stake_account = next_account_info(iter)?;
    
    if !stake_account.is_writable {
        msg!("Stake account is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    if stake_account.owner != program_id {
        msg!("Stake account is not owned by the program");
        return Err(ProgramError::InvalidAccountData);
    }
    
    let user_reward_token_account = next_account_info(iter)?;
    
    if !user_reward_token_account.is_writable {
        msg!("User Reward Token Account is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let reward_mint_account = next_account_info(iter)?;
    
    if !reward_mint_account.is_writable {
        msg!("Reward Mint Account is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let stake_details_account = next_account_info(iter)?;
    
    let nft_mint_account = next_account_info(iter)?;
    let token_program = next_account_info(iter)?;

    if stake_account.owner != program_id {
        msg!("Stake Account not owned by the program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if stake_details_account.owner != program_id {
        msg!("Stake Details Account not owned by the Program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let seeds = [
        b"stake", 
        stake_details_account.key.as_ref(), 
        nft_mint_account.key.as_ref(), 
        user_account.key.as_ref()
    ];

    let(stake_account_key, _bump) = Pubkey::find_program_address(&seeds, program_id);

    if stake_account_key != *stake_account.key {
        msg!("Stake Account Mismatch !");
        msg!("Expected : {}, received : {}", stake_account_key, stake_account.key);
        return Err(ProgramError::InvalidAccountData);
    }

    if stake_account.data.borrow().len() == 0 {
        msg!("Stake Account is not initialized");
        return Err(NftStakingError::AccountNotInitialized.into());
    }

    let mut stake_account_data = Stake::try_from_slice(&*stake_account.data.borrow())?;

    let user_reward_ata = get_associated_token_address_with_program_id(
        user_account.key,
        reward_mint_account.key, 
        token_program.key
    );

    if user_reward_ata != *user_reward_token_account.key{
        msg!("Invalid Reward Token Account");
        msg!("Expected : {}, received : {}", user_reward_ata, user_reward_token_account.key);
        return Err(ProgramError::InvalidAccountData);
    }

    if stake_account_data.reward_mint_ata != *user_reward_token_account.key {
        msg!("Reward Token Account Mismatch !");
        msg!("Expected : {}, received : {}", stake_account_data.reward_mint_ata, user_reward_token_account.key);
        return Err(ProgramError::InvalidAccountData);
    }

    let reward_mint_data = Mint::unpack(&*reward_mint_account.data.borrow())?;

    let current_time = Clock::get()?.unix_timestamp;

    let staked_duration = current_time - stake_account_data.staked_at;
    let reward_token_amount = (staked_duration * 1000) as u64; // 1000 reward tokens released per second 

    msg!("Staked Duration is {} seconds", staked_duration);
    msg!("minting {} tokens to {}",reward_token_amount, user_reward_token_account.key);

    let ix = mint_to_checked(
        token_program.key, 
        reward_mint_account.key, 
        user_reward_token_account.key, 
        stake_details_account.key,     // as stake_details is the mint authority of the reward token
        &[stake_details_account.key], // as the stake details can only mint the reward tokens 
        reward_token_amount, 
        reward_mint_data.decimals,
    )?;

    let stake_details_data = StakeDetails::try_from_slice(&*stake_details_account.data.borrow())?;

    let signers_seeds = [
        b"stake_details",
        stake_details_data.creator.as_ref(),
        stake_details_data.collection_mint.as_ref(),
        &[stake_details_data.bump_seed]
    ];

    invoke_signed(
        &ix, 
        &[
            stake_details_account.clone(),
            user_reward_token_account.clone(),
            reward_mint_account.clone(),
        ], 
        &[&signers_seeds]
    )?;

    // now reset the staked_at

    let now = Clock::get()?.unix_timestamp;

    stake_account_data.staked_at = now;
    stake_account_data.serialize(&mut *stake_account.data.borrow_mut())?;

    msg!("Successfully reset the staked_at to : {}", now);

    Ok(())  
}