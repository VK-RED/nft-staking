use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct StakeDetails {
    pub creator: Pubkey,
    pub reward_token_mint: Pubkey,
    pub collection_mint: Pubkey,
    pub bump_seed: u8,
}

impl StakeDetails {
    // As each data is 32 bytes
    pub const LEN:usize = 32 + 32 + 32 + 1;
}
