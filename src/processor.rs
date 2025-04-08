use solana_program::{
    account_info::AccountInfo, 
    entrypoint::ProgramResult, 
    msg, 
    pubkey::Pubkey};

pub fn process_instruction(
    program_id: &Pubkey, 
    accounts_info: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult{
    
    msg!("Hello world");
    Ok(())
}