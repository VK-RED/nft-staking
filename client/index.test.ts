import { createMint, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Connection, Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, TransactionInstruction, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import {describe, it} from  "bun:test";
import { createNft, mplTokenMetadata } from "@metaplex-foundation/mpl-token-metadata";
import { generateSigner, keypairIdentity, percentAmount } from "@metaplex-foundation/umi";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";


describe("nft-staking", async()=>{

    const clusterUrl = "http://localhost:8899";
    const connection = new Connection(clusterUrl);

    const PROGRAM_ID = new PublicKey("8bJiirYL3y3Gw1W2dwnpwUf3fyMoQmhF4TwYKEyuZkwt");

    const keypair = Keypair.generate();
    console.log("Keypair is : ", keypair.publicKey.toBase58());
    
    console.log("Air Dropping the Keypair 5 SOL");
    const tx = await connection.requestAirdrop(keypair.publicKey, 5*LAMPORTS_PER_SOL)
    await connection.confirmTransaction(tx, "finalized");
    console.log("AIRDROP COMPLETED !!");

    let masterNft: PublicKey|null = null;

    const createNftCollection = async () => {
        const umi = createUmi(clusterUrl);
        // convert to Umi compatible keypair
        const umiKeypair = umi.eddsa.createKeypairFromSecretKey(keypair.secretKey);

        // load the MPL metadata program plugin and assign a signer to our umi instance
        umi.use(keypairIdentity(umiKeypair))
        .use(mplTokenMetadata());

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
        
        
        return collectionMint;
    }

    const getStakeDetailsAccount = (user:PublicKey, collectionMint:PublicKey)=>{
        const [stakeDetailsAccount] = PublicKey.findProgramAddressSync(
            [Buffer.from("stake_details"), user.toBuffer(), collectionMint.toBuffer()],
            PROGRAM_ID,
        );
        return stakeDetailsAccount;
    }

    it("init", async ()=>{

        // create a reward token and set the keypair as the mint authority
        // create an nft with a collection
        // find the stake details account

        const rewardMint = await createMint(connection, keypair, keypair.publicKey, null, 9, );
        console.log("Reward Mint Address : ", rewardMint.toBase58());
        const collectionMint = await createNftCollection();

        const collectionMintKey = new PublicKey(collectionMint.publicKey.toString());
        masterNft = collectionMintKey;

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

        console.log("Recent Blockhash : ", recentBlockhash);

        const v0Message = new TransactionMessage({
            instructions: [ix],
            payerKey: keypair.publicKey,
            recentBlockhash
        }).compileToV0Message();

        const tx = new VersionedTransaction(v0Message);
        tx.sign([keypair]);

        const sig = await connection.sendTransaction(tx, {skipPreflight: true, maxRetries: 10});
        console.log("Init Signature : ", sig);
        
    })

})