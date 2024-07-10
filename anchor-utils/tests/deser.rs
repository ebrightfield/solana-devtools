use solana_devtools_anchor_utils::deserialize::IdlWithDiscriminators;

fn test_idl_accounts_borsh_serde_json(account_data: &[u8], deser: &IdlWithDiscriminators) {
    // borsh-deserialized to [Value]
    let (type_def, deser_act) = deser.try_account_data_to_value_inner(account_data).unwrap();

    // Back to borsh-serialized bytes
    let serialized_data = deser.try_from_value_borsh(type_def, &deser_act).unwrap();
    // Compare byte-for-byte, but allow for zeroed space on `Option::None``
    for i in 0..serialized_data.len() {
        if let Some(value) = account_data.get(i) {
            assert!(
                serialized_data[i] == *value || serialized_data[i] == 0,
                "index {} differs between\naccount data {:?}\nand serialized data {:?}\n, {} (expected account data) != {}",
                i,
                &account_data[..i],
                &serialized_data[..i],
                *value,
                serialized_data[i],
            );
        } else {
            panic!(
                "account data is shorter than what we serialized {} < {}",
                account_data.len(),
                serialized_data.len(),
            );
        }
    }

    // Which borsh-deserializes to the same [Value]
    let deserialized_again = deser
        .try_account_data_to_value_inner(&serialized_data)
        .unwrap();
    assert_eq!(deserialized_again.1, deser_act);

    // Which borsh-serializes to the same bytes
    let serialized_data_again = deser
        .try_from_value_borsh(type_def, &deserialized_again.1)
        .unwrap();
    assert_eq!(serialized_data, serialized_data_again);
}

#[test]
fn deserialize_and_serialize_value_to_borsh() {
    let marinade_idl =
        IdlWithDiscriminators::from_file("tests/fixtures/marinade_idl.json").unwrap();
    let tcomp_idl = IdlWithDiscriminators::from_file("tests/fixtures/tcomp_idl.json").unwrap();

    let marinade_state = include_bytes!("../tests/fixtures/marinade_state.bin");
    let tcomp_bid_state = include_bytes!("../tests/fixtures/tcomp_bid_state.bin");
    test_idl_accounts_borsh_serde_json(marinade_state, &marinade_idl);
    test_idl_accounts_borsh_serde_json(tcomp_bid_state, &tcomp_idl);
}
