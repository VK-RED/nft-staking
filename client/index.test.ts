import { ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccount, createMint, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Connection, Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, Transaction, TransactionInstruction, TransactionMessage, VersionedTransaction, type ParsedAccountData } from "@solana/web3.js";
import {describe, expect, it} from  "bun:test";
import { createNft, fetchMetadataFromSeeds, mplTokenMetadata, verifyCollectionV1 } from "@metaplex-foundation/mpl-token-metadata";
import { generateSigner, keypairIdentity, percentAmount, type KeypairSigner } from "@metaplex-foundation/umi";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { Stake, StakeDetails } from "./utils";

// Replace with your program id
const PROGRAM_ID = new PublicKey("8bJiirYL3y3Gw1W2dwnpwUf3fyMoQmhF4TwYKEyuZkwt");

describe("nft-staking", async()=>{

    const clusterUrl = "http://localhost:8899";
    const connection = new Connection(clusterUrl);

    const keypair = Keypair.generate();
    console.log("Keypair is : ", keypair.publicKey.toBase58());
    
    console.log("Air Dropping the Keypair 5 SOL");
    const tx = await connection.requestAirdrop(keypair.publicKey, 5*LAMPORTS_PER_SOL)
    await connection.confirmTransaction(tx, "finalized");
    console.log("AIRDROP COMPLETED !!");

    let umiMasterNft: KeypairSigner|null = null;
    let stakeDetailsKey: PublicKey|null = null;
    let rewardAccount: PublicKey | null = null;
    let nftMintAccount: PublicKey | null = null;

    const umi = createUmi(clusterUrl);
    // convert to Umi compatible keypair
    const umiKeypair = umi.eddsa.createKeypairFromSecretKey(keypair.secretKey);

    // load the MPL metadata program plugin and assign a signer to our umi instance
    umi.use(keypairIdentity(umiKeypair))
    .use(mplTokenMetadata());

    const createNftCollection = async () => {

        const collectionMint = generateSigner(umi);

        // create and mint NFT
        await createNft(umi, {
            mint: collectionMint,
            name: "NFT Collection",
            uri:"https://solana.com",
            updateAuthority: umi.identity.publicKey,
            sellerFeeBasisPoints: percentAmount(0),
            isCollection: true,
        }).sendAndConfirm(umi, { send: { commitment: "finalized" } });
        
        umiMasterNft = collectionMint;
        console.log("Collection Mint is : ", collectionMint.publicKey);
        
        return collectionMint;
    }

    const mintNft = async () => {
        if(!umiMasterNft){
            throw new Error("Master NFT not initialized");
        }

        const nftMint = generateSigner(umi);
        console.log("Nft Mint is : ", nftMint.publicKey);

        await createNft(umi, {
            mint: nftMint,
            name: "NFT",
            uri:"https://solana.com",
            updateAuthority: umi.identity.publicKey,
            sellerFeeBasisPoints: percentAmount(0),
            collection:{
                verified:false, 
                key: umiMasterNft.publicKey
            }
        }).sendAndConfirm(umi, { send: { commitment: "finalized" } });

        // verify the nft

        const nftMetaData = await fetchMetadataFromSeeds(umi,{mint:nftMint.publicKey});

        await verifyCollectionV1(umi, {
            metadata: nftMetaData.publicKey,
            collectionMint: umiMasterNft.publicKey,
            authority: umi.identity,
        }).sendAndConfirm(umi);

        console.log("NFT Verified Successfully");

        return nftMint;
    }

    const getStakeDetailsAccount = (user:PublicKey, collectionMint:PublicKey)=>{
        const [stakeDetailsAccount] = PublicKey.findProgramAddressSync(
            [Buffer.from("stake_details"), user.toBuffer(), collectionMint.toBuffer()],
            PROGRAM_ID,
        );
        stakeDetailsKey = stakeDetailsAccount;
        return stakeDetailsAccount;
    }


    it("should init staking", async ()=>{

        // create a reward token and set the keypair as the mint authority
        // create an nft with a collection
        // find the stake details account

        const rewardMint = await createMint(connection, keypair, keypair.publicKey, null, 9, );
        rewardAccount = rewardMint;
        console.log("Reward Mint Address : ", rewardMint.toBase58());
        const collectionMint = await createNftCollection();

        const collectionMintKey = new PublicKey(collectionMint.publicKey.toString());

        console.log("Collection Mint Address : ", collectionMint.publicKey);
        const stakeDetailsAccount = getStakeDetailsAccount(keypair.publicKey, collectionMintKey);
        console.log("Stake Details Account : ", stakeDetailsAccount.toBase58());    

        const ix = new TransactionInstruction({
            keys: [
                {pubkey: keypair.publicKey, isSigner: true, isWritable: true},
                {pubkey: rewardMint, isSigner: false, isWritable: true},
                {pubkey: collectionMintKey, isSigner: false, isWritable: false},
                {pubkey: stakeDetailsAccount, isSigner: false, isWritable: true},
                {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
                {pubkey: SystemProgram.programId, isSigner: false, isWritable: false},
            ],
            data: Buffer.from([0]),
            programId: PROGRAM_ID,
        })


        const recentBlockhash = (await connection.getLatestBlockhash('confirmed')).blockhash;

        const v0Message = new TransactionMessage({
            instructions: [ix],
            payerKey: keypair.publicKey,
            recentBlockhash
        }).compileToV0Message();

        const tx = new VersionedTransaction(v0Message);
        tx.sign([keypair]);

        const sig = await connection.sendTransaction(tx, {skipPreflight: true, maxRetries: 10});
        console.log("Init Signature : ", sig);

        const {blockhash, lastValidBlockHeight} = (await connection.getLatestBlockhash());
        await connection.confirmTransaction({blockhash, signature:sig, lastValidBlockHeight }, 'finalized');

        const accountDetails = await connection.getParsedAccountInfo(stakeDetailsAccount);
        const serialized = accountDetails.value?.data;

        if(!serialized){
            throw new Error("Stake Details Account not found");
        }

        const stakeDetails = StakeDetails.getDeserialized(serialized as Buffer);

        expect(stakeDetails.creator).toBe(keypair.publicKey.toBase58());
        expect(stakeDetails.reward_token_mint).toBe(rewardMint.toBase58());
        expect(stakeDetails.collection_mint).toBe(collectionMint.publicKey);
        
    })

    it("should stake NFT", async() => {

        if(!stakeDetailsKey || !rewardAccount){
            throw new Error("Stake Details Account or Reward Account not initialized");
        }

        const nftMint = await mintNft();        
        const nftMetaData = await fetchMetadataFromSeeds(umi,{mint:nftMint.publicKey} );

        nftMintAccount = new PublicKey(nftMint.publicKey.toString());
        const nftMetaDataKey = new PublicKey(nftMetaData.publicKey.toString());
        
        const userTokenAccount = findAssociatedTokenAddress(
            keypair.publicKey,
            nftMintAccount
        );

        const stakeAccount = findStakeAccount(stakeDetailsKey, nftMintAccount, keypair.publicKey);
        const stakeTokenAccount = findAssociatedTokenAddress(stakeAccount, nftMintAccount); 

        const userRewardAccount = await createAssociatedTokenAccount(
            connection, 
            keypair, 
            rewardAccount, 
            keypair.publicKey,
        )

        console.log("User Token Account : ", userTokenAccount.toBase58());
        console.log("Stake Account : ", stakeAccount.toBase58());
        console.log("Stake Token Account : ", stakeTokenAccount.toBase58());
        console.log("User Reward Account : ", userRewardAccount.toBase58());

        const ix = new TransactionInstruction({
            keys:[
                {pubkey: keypair.publicKey, isSigner: true, isWritable: true},
                {pubkey: nftMintAccount, isSigner: false, isWritable: false},
                {pubkey: nftMetaDataKey, isSigner: false, isWritable: false},
                {pubkey: userTokenAccount, isSigner: false, isWritable: true},
                {pubkey: userRewardAccount, isSigner: false, isWritable: true},
                {pubkey: stakeDetailsKey, isSigner: false, isWritable: false},
                {pubkey: stakeAccount, isSigner: false, isWritable: true},
                {pubkey: stakeTokenAccount, isSigner: false, isWritable: true},
                {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
                {pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
                {pubkey: SystemProgram.programId, isSigner: false, isWritable: false},
            ],
            data: Buffer.from([1]),
            programId: PROGRAM_ID,
        });

        const tx = new Transaction().add(ix);

        const stakeSig = await connection.sendTransaction(tx, [keypair]);
        console.log("Stake Signature : ", stakeSig);

        const {blockhash, lastValidBlockHeight} = (await connection.getLatestBlockhash());
        await connection.confirmTransaction({blockhash, signature:stakeSig, lastValidBlockHeight }, 'finalized');

        const stakeOnchainData = (await connection.getParsedAccountInfo(stakeAccount)).value?.data;
        if(!stakeOnchainData){
            throw new Error("Stake Account Data not found");
        }

        const stake = Stake.getDeserialized(stakeOnchainData as Buffer);

        expect(stake.stake_details_key).toBe(stakeDetailsKey.toBase58());
        expect(stake.nft_mint).toBe(nftMintAccount.toBase58());
        expect(stake.reward_mint).toBe(rewardAccount.toBase58());
        expect(stake.reward_mint_ata).toBe(userRewardAccount.toBase58());


        const stakeTokenAccountData = (await connection.getParsedAccountInfo(stakeTokenAccount)).value?.data as ParsedAccountData;
        expect(stakeTokenAccountData.parsed.info.tokenAmount.amount).toBe("1");

    })

    it("should claim rewards", async () => {
        if(!stakeDetailsKey || !rewardAccount || !nftMintAccount){
            throw new Error("Stake Details Account or Reward Account or NFT Mint not initialized");
        }

        const REWARD_PER_SECOND = 1000;

        console.log("Waiting for 1.5 seconds to overcome lockin period");
        await new Promise(resolve => setTimeout(resolve, 1500));
        console.log("Done waiting");

        const stakeAccount = findStakeAccount(stakeDetailsKey, nftMintAccount, keypair.publicKey);

        const userRewardAccount = findAssociatedTokenAddress(
            keypair.publicKey, 
            rewardAccount, 
        )

        const rewardBeforeClaim = await connection.getParsedAccountInfo(userRewardAccount);
        const stakeAccountBeforeClaim = await connection.getParsedAccountInfo(stakeAccount);

        if(!rewardBeforeClaim || !stakeAccountBeforeClaim){
            throw new Error("User Reward Account Data or Stake Account Data not found");
        }

        const stakeAccountBeforeClaimData = Stake.getDeserialized(stakeAccountBeforeClaim.value?.data as Buffer);

        let ix = new TransactionInstruction({
            keys:[
                {pubkey: keypair.publicKey, isSigner: true, isWritable: false},
                {pubkey: stakeAccount, isSigner: false, isWritable: true},
                {pubkey: userRewardAccount, isSigner: false, isWritable: true},
                {pubkey: rewardAccount, isSigner: false, isWritable: true},
                {pubkey: stakeDetailsKey, isSigner: false, isWritable: false},
                {pubkey: nftMintAccount, isSigner: false, isWritable: false},
                {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
            ],
            data: Buffer.from([2]),
            programId: PROGRAM_ID,
        });
        
        const tx = new Transaction().add(ix);

        const sig = await connection.sendTransaction(tx, [keypair]);

        console.log("Claim Signature : ", sig);

        const {blockhash, lastValidBlockHeight} = (await connection.getLatestBlockhash());
        await connection.confirmTransaction({blockhash, signature:sig, lastValidBlockHeight }, 'finalized');


        const rewardAfterClaim = await connection.getParsedAccountInfo(userRewardAccount);
        const stakeAccountAfterClaim = await connection.getParsedAccountInfo(stakeAccount);

        if(!rewardAfterClaim || !stakeAccountAfterClaim){
            throw new Error("User Reward Account Data or Stake Account Data not found");
        }

        const rewardAfterClaimData = rewardAfterClaim.value?.data as ParsedAccountData;
        const stakeAccountAfterClaimData = Stake.getDeserialized(stakeAccountAfterClaim.value?.data as Buffer);

        expect(stakeAccountAfterClaimData.staked_at).toBeGreaterThan(stakeAccountBeforeClaimData.staked_at);    

        const stakedDuration = Number(stakeAccountAfterClaimData.staked_at - stakeAccountBeforeClaimData.staked_at);
        const expectedRewardAmount = stakedDuration * REWARD_PER_SECOND;

        expect(rewardAfterClaimData.parsed.info.tokenAmount.amount).toBe(expectedRewardAmount.toString());
    })

    it("should unstake NFT", async()=>{
        if(!stakeDetailsKey || !rewardAccount || !nftMintAccount){
            throw new Error("Stake Details Account or Reward Account or NFT Mint not initialized");
        }

        console.log("Waiting for 1.5 seconds to overcome lockin period");
        await new Promise(resolve => setTimeout(resolve, 1500));
        console.log("Done waiting");

        const userNftTokenAccount =  findAssociatedTokenAddress(
            keypair.publicKey,
            nftMintAccount
        );

        const stakeAccount = findStakeAccount(stakeDetailsKey, nftMintAccount, keypair.publicKey);
        const stakeTokenAccount = findAssociatedTokenAddress(stakeAccount, nftMintAccount); 

        const userRewardAccount = findAssociatedTokenAddress(
            keypair.publicKey, 
            rewardAccount, 
        )

        const rewardBeforeUnStake = await connection.getParsedAccountInfo(userRewardAccount);
        const stakeAccountBeforeUnStake = await connection.getParsedAccountInfo(stakeAccount);
        const userAccountBeforeUnStake = await connection.getParsedAccountInfo(keypair.publicKey);
        const stakeNftAccountBeforeUnStake = await connection.getParsedAccountInfo(nftMintAccount);
        
        const userBalanceBeforeUnStake = userAccountBeforeUnStake.value?.lamports || 0;
        const stakeBalanceBeforeUnStake = stakeAccountBeforeUnStake.value?.lamports || 0;
        const stakeNftAccountBalanceBeforeUnStake = stakeNftAccountBeforeUnStake.value?.lamports || 0;

        if(!rewardBeforeUnStake || !stakeAccountBeforeUnStake || !userAccountBeforeUnStake){
            throw new Error("User Reward Account Data or Stake Account Data or User Account Data not found");
        }

        const rewardBeforeUnStakeData = rewardBeforeUnStake.value?.data as ParsedAccountData;


        let ix = new TransactionInstruction({
            keys:[
                {pubkey: keypair.publicKey, isSigner: true, isWritable: true},
                {pubkey: userNftTokenAccount, isSigner: false, isWritable: true},
                {pubkey: userRewardAccount, isSigner: false, isWritable: true},
                {pubkey: stakeDetailsKey, isSigner: false, isWritable: false},
                {pubkey: stakeAccount, isSigner: false, isWritable: true},
                {pubkey: stakeTokenAccount, isSigner: false, isWritable: true},
                {pubkey: nftMintAccount, isSigner: false, isWritable: false},
                {pubkey: rewardAccount, isSigner: false, isWritable: true},
                {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
            ],
            data: Buffer.from([3]),
            programId: PROGRAM_ID,
        });
        
        const tx = new Transaction().add(ix);

        const sig = await connection.sendTransaction(tx, [keypair]);

        console.log("Unstake Signature : ", sig);

        const {blockhash, lastValidBlockHeight} = (await connection.getLatestBlockhash());
        await connection.confirmTransaction({blockhash, signature:sig, lastValidBlockHeight }, 'finalized');

        const parsedTx = await connection.getParsedTransaction(sig, {commitment: 'finalized'});
        const txFee = parsedTx?.meta?.fee || 0;

        const rewardAfterUnStake = await connection.getParsedAccountInfo(userRewardAccount);
        const userNftAccountInfo = await connection.getParsedAccountInfo(userNftTokenAccount);
        const userAccountAfterUnStake = await connection.getParsedAccountInfo(keypair.publicKey);

        const stakeAccountAfterUnStake = await connection.getParsedAccountInfo(stakeAccount);
        const stakeNftAccountAfterUnStake = await connection.getParsedAccountInfo(stakeTokenAccount);


        if(!userAccountAfterUnStake || !userNftAccountInfo || !rewardAfterUnStake){
            throw new Error("User Account Data or User NFT Account or Reward Account Data not found");
        }

        const userNftAccountData = userNftAccountInfo.value?.data as ParsedAccountData;
        const rewardAfterUnStakeData = rewardAfterUnStake.value?.data as ParsedAccountData;
        const expectedUserBalance = userBalanceBeforeUnStake - txFee + stakeBalanceBeforeUnStake + stakeNftAccountBalanceBeforeUnStake;
        
        expect(stakeAccountAfterUnStake.value).toBeNull();
        expect(stakeNftAccountAfterUnStake.value).toBeNull();
        expect(Number(rewardAfterUnStakeData.parsed.info.tokenAmount.amount)).toBeGreaterThan(Number(rewardBeforeUnStakeData.parsed.info.tokenAmount.amount));
        expect(userNftAccountData.parsed.info.tokenAmount.uiAmount).toBe(1);
        expect(userAccountAfterUnStake.value?.lamports || 0).toBeGreaterThan(expectedUserBalance);

    })

})

function findAssociatedTokenAddress(
    walletAddress: PublicKey,
    tokenMintAddress: PublicKey
): PublicKey {
    return PublicKey.findProgramAddressSync(
        [
            walletAddress.toBuffer(),
            TOKEN_PROGRAM_ID.toBuffer(),
            tokenMintAddress.toBuffer(),
        ],
        ASSOCIATED_TOKEN_PROGRAM_ID
    )[0];
}

const findStakeAccount = (stakeDetailsKey:PublicKey, nftMintKey:PublicKey, userKey:PublicKey) => {
    const [stakeAccount] = PublicKey.findProgramAddressSync(
        [
            Buffer.from("stake"),
            stakeDetailsKey.toBuffer(),
            nftMintKey.toBuffer(),
            userKey.toBuffer(),
        ],
        PROGRAM_ID
    );

    return stakeAccount;
}