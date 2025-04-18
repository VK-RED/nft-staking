use solana_program::{
    account_info::AccountInfo, 
    entrypoint::ProgramResult, 
    msg, 
    program_error::ProgramError, 
    pubkey::Pubkey
};

use crate::instruction;

pub enum StakingInstruction {
    Initialize, 
    Stake,
    ClaimRewards
}

impl StakingInstruction {
    pub fn unpack(instruction_data: &[u8]) -> Result<Self, ProgramError>{

        let (variant, _rest) = instruction_data.split_first().ok_or(ProgramError::InvalidInstructionData)?;

        match *variant {
            0 => {
                Ok(Self::Initialize)
            },
            1 => {
                Ok(Self::Stake)
            },
            2 => {
                Ok(Self::ClaimRewards)
            },
            _ => {
                msg!("No Instruction is set for the variant : {}", variant);
                Err(ProgramError::InvalidInstructionData)
            }
        }
    }
}

pub fn process_instruction(
    program_id: &Pubkey, 
    accounts_info: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult{

    match StakingInstruction::unpack(instruction_data)? {

        StakingInstruction::Initialize => {
            instruction::init_staking(program_id, accounts_info)?
        }

        StakingInstruction::Stake =>{ 
            instruction::stake(program_id, accounts_info)?
        }

        StakingInstruction::ClaimRewards => {
            instruction::claim_rewards(program_id, accounts_info)?
        }

    }

    
    Ok(())
}