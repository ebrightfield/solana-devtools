import * as anchor from "@project-serum/anchor";

import * as testUserJsonJson from "./test_user.json";
export const testUserJson = new anchor.web3.PublicKey(testUserJsonJson.pubkey);
import * as mintJsonJson from "./mint.json";
export const mintJson = new anchor.web3.PublicKey(mintJsonJson.pubkey);
import * as testUserTokenActJsonJson from "./test_user_token_act.json";
export const testUserTokenActJson = new anchor.web3.PublicKey(testUserTokenActJsonJson.pubkey);