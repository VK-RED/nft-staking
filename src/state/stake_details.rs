use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct StakeDetails {
    pub creator: Pubkey,
    pub reward_token_mint: Pubkey,
    pub collection_mint: Pubkey,
}

impl StakeDetails {
    // As each data is 32 bytes
    pub const LEN:usize = 32 + 32 + 32;

    pub fn get_serialized(&self) -> Result<Vec<u8>, ProgramError> {
        let serialized = borsh::to_vec(self)?;
        Ok(serialized)
    }
}
