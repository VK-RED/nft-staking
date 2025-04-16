use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Stake {
    pub stake_details_key : Pubkey,
    pub nft_mint : Pubkey,
    pub reward_mint: Pubkey,
    pub reward_mint_ata: Pubkey,
    pub staked_at: i64,
}

impl Stake {
    pub const LEN: usize = 32 + 32 + 32 + 32 + 8;
}