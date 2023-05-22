import * as anchor from "@project-serum/anchor";

import * as testUserJson from "./test_user.json";
export const testUser = new anchor.web3.PublicKey(testUserJson.pubkey);
import * as mintJson from "./mint.json";
export const mint = new anchor.web3.PublicKey(mintJson.pubkey);
import * as usdcMintJson from "./usdc_mint.json";
export const usdcMint = new anchor.web3.PublicKey(usdcMintJson.pubkey);
import * as testUserTokenActJson from "./test_user_token_act.json";
export const testUserTokenAct = new anchor.web3.PublicKey(testUserTokenActJson.pubkey);