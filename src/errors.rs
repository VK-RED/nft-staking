use thiserror::Error;
use solana_program::program_error::ProgramError;

#[derive(Error, Debug)]
pub enum NftStakingError{
    #[error("The Collection NFT does not match with the Collection NFT in the Staking Details")]
    CollectionMintMismatch,

    #[error("The given NFT does not belongs to any Collection")]
    NoCollectionSet,

    #[error("The metadata account does not point to the given nft mint")]
    InvalidMetadataAccount,

    #[error("NFT not verified")]
    NftNotVerified,

    #[error("No NFT found in the Token Account")]
    NftEmpty
}

impl From<NftStakingError> for ProgramError{
    fn from(e: NftStakingError) -> Self {
        ProgramError::Custom(e as u32)
    }
}