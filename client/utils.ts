import { PublicKey } from "@solana/web3.js";
import * as borsh from "borsh";

const PubkeySchema = {
    array:{
        type: "u8",
        len: 32
    }
};

const getPubkeyBase58 = (data:Uint8Array) => {
    return new PublicKey(data).toBase58();
}

export class Stake{
    stake_details_key : string;
    nft_mint : string;
    reward_mint: string;
    reward_mint_ata: string;
    staked_at: number;

    constructor(stake_details_key:string, nft_mint:string, reward_mint:string, reward_mint_ata:string, staked_at:number){
        this.stake_details_key = stake_details_key;
        this.nft_mint = nft_mint;
        this.reward_mint = reward_mint;
        this.reward_mint_ata = reward_mint_ata;
        this.staked_at = staked_at;
    }

    static getDeserialized(data:Buffer){

        const deserialized = borsh.deserialize({
            struct:{
                stake_details_key:PubkeySchema,
                nft_mint: PubkeySchema,
                reward_mint: PubkeySchema, 
                reward_mint_ata:PubkeySchema,
                staked_at: 'i64'
            }
        },data);


        const stake = new Stake(
            // @ts-ignore
            getPubkeyBase58(deserialized.stake_details_key),
            // @ts-ignore
            getPubkeyBase58(deserialized.nft_mint),
            // @ts-ignore
            getPubkeyBase58(deserialized.reward_mint),
            // @ts-ignore
            getPubkeyBase58(deserialized.reward_mint_ata),
            // @ts-ignore
            deserialized.staked_at
        )

        return stake;
    }
}

export class StakeDetails{
    creator: string;
    reward_token_mint: string;
    collection_mint: string;
    bump_seed: number;

    constructor(creator:string, reward_token_mint:string, collection_mint:string, bump_seed:number){
        this.bump_seed = bump_seed;
        this.creator = creator;
        this.reward_token_mint = reward_token_mint;
        this.collection_mint = collection_mint;
    }

    static getDeserialized(data:Buffer){
        const deserialized = borsh.deserialize({
            struct:{
                creator: PubkeySchema,
                reward_token_mint: PubkeySchema,
                collection_mint: PubkeySchema,
                bump_seed: 'u8',
            }
        },data);

        const stakeDetails = new StakeDetails(
            // @ts-ignore
            getPubkeyBase58(deserialized.creator),
            // @ts-ignore
            getPubkeyBase58(deserialized.reward_token_mint),
            // @ts-ignore
            getPubkeyBase58(deserialized.collection_mint),
            // @ts-ignore
            deserialized.bump_seed
        )

        return stakeDetails;
    }
}