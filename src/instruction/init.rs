use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo}, entrypoint::ProgramResult, msg, program::{invoke, invoke_signed}, program_error::ProgramError, pubkey::Pubkey, rent::Rent, system_instruction, sysvar::Sysvar
};
use spl_token::instruction::{set_authority, AuthorityType};
use crate::state::StakeDetails;


pub fn init_staking(program_id: &Pubkey,accounts: &[AccountInfo]) -> ProgramResult{

    let iter = &mut accounts.iter();

    // isSigner and isWritable
    let user = next_account_info(iter)?;

    if !user.is_signer || !user.is_writable {
        msg!("User account is not signer or writable");
        return Err(ProgramError::InvalidAccountData);
    }

    // isWritable
    let reward_mint = next_account_info(iter)?;

    if !reward_mint.is_writable {
        msg!("Reward Mint is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let collection_mint = next_account_info(iter)?;

    // isWritable
    let stake_details_acc = next_account_info(iter)?;
    
    if !stake_details_acc.is_writable {
        msg!("Stake Details Account is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    let token_program = next_account_info(iter)?;

    let system_program = next_account_info(iter)?;

    // create a pda 
    let (stake_details_key, stake_acc_bump) = Pubkey::find_program_address(
        &[b"stake_details", 
        user.key.as_ref(), 
        collection_mint.key.as_ref()],
        program_id
    );

    if stake_details_key != *stake_details_acc.key {
        msg!("Stake Details Key Mismatch");
        msg!("Stake Details Key expected : {}, received : {}", stake_details_key, stake_details_acc.key);
        return Err(ProgramError::InvalidAccountData);
    }

    if stake_details_acc.data.borrow().len() != 0 {
        msg!("Stake Details Account is already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let space = StakeDetails::LEN;
    let lamports = Rent::get()?.minimum_balance(space);

    let instruction =system_instruction::create_account(
        user.key, 
        &stake_details_key, 
        lamports, 
        space as u64, 
        program_id
    );

    let account_infos = [user.clone(), stake_details_acc.clone(), system_program.clone()];
    let seeds = [b"stake_details", user.key.as_ref(), collection_mint.key.as_ref(), &[stake_acc_bump]];

    invoke_signed(
        & instruction, 
        & account_infos, 
        &[&seeds]
    )?;

    msg!("Successfully Created Stake Details Account");

    let stake_details = StakeDetails{
        creator: *user.key,
        reward_token_mint: *reward_mint.key,
        collection_mint: *collection_mint.key,
    };

    stake_details.serialize(&mut *stake_details_acc.data.borrow_mut())?;

    msg!("Successfully written stake_details in onchain");

    let mint_authority_ix = set_authority(
        token_program.key, 
        reward_mint.key, 
        Some(stake_details_acc.key), 
        AuthorityType::MintTokens,   // set as mint authority
        user.key, 
        &[user.key]
    )?;

    invoke( 
        &mint_authority_ix,
        &[
            user.clone(), 
            stake_details_acc.clone(), 
            token_program.clone(), 
            reward_mint.clone()
        ]
    )?;

    msg!("Successfully transfered mint authority to stake details account");

    Ok(())

}