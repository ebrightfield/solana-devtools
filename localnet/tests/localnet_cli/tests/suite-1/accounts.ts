import * as anchor from "@project-serum/anchor";

import * as testUserJson from "./test_user";
export const testUser = new anchor.web3.PublicKey(testUserJson.pubkey);
import * as mintJson from "./mint";
export const mint = new anchor.web3.PublicKey(mintJson.pubkey);
import * as testUserTokenActJson from "./test_user_token_act";
export const testUserTokenAct = new anchor.web3.PublicKey(testUserTokenActJson.pubkey);