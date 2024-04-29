use solana_devtools_macros::const_data;
use solana_sdk::{pubkey, pubkey::Pubkey};

#[const_data(
    "VAR1", "foo", 84u64;
    "VAR2", "bar", 87u64;
    "VAR3", "baz", 89u64;
)]
struct TypeWithCode {
    field_a: &'static str,
    field_b: u64,
    #[code]
    code: u32,
}

#[const_data(
    "VAR4", "yes", 17u64;
    "VAR5", "no", 18u64;
    "VAR6", "maybe", 19u64;
)]
struct TypeWithName {
    #[name]
    foo: &'static str,
    field_a: &'static str,
    field_b: u64,
}

#[const_data(
    "VAR7", "yes", pubkey!("9ykQgmRHR4EsCPRaMQCMWoa58QqWXEw2fSQ2LkVCHXdd");
    "VAR8", "no", pubkey!("EULQ7RXBmMideABHPYz4ifk4cfNuuWNMBMAod8ZQxXFa");
    "VAR9", "maybe", pubkey!("C6YGW51NQ6mJjpuoihLxgDKZSQgd8roQA6bkBkgDNNMz");
)]
struct TypeWithNameAndCode {
    #[name]
    foo: &'static str,
    field_a: &'static str,
    field_b: Pubkey,
    #[code]
    field_c: u32,
}

mod type_without_code {
    use solana_devtools_macros::const_data;

    #[const_data(
        "VAR1_NOCODE", "foo", 83u64;
        "VAR2_NOCODE", "bar", 86u64;
    )]
    pub struct TypeWithoutCode {
        pub field_a: &'static str,
        pub field_b: u64,
    }
}
#[test]
fn const_data_works() {
    assert_eq!(type_without_code::VAR1_NOCODE.field_b, 83);
    assert_eq!(type_without_code::VAR2_NOCODE.field_b, 86);
    assert_eq!(type_without_code::VAR1_NOCODE.field_a, "foo");
    assert_eq!(type_without_code::VAR2_NOCODE.field_a, "bar");
    assert_eq!(type_without_code::TypeWithoutCode::NUM_CONSTS, 2usize);

    assert_eq!(TypeWithCode::NUM_CONSTS, 3usize);
    assert_eq!(VAR1.field_a, "foo");
    assert_eq!(VAR2.field_a, "bar");
    assert_eq!(VAR3.field_a, "baz");
    assert_eq!(VAR1.field_b, 84);
    assert_eq!(VAR2.field_b, 87);
    assert_eq!(VAR3.field_b, 89);
    assert_eq!(VAR1.code, 0);
    assert_eq!(VAR2.code, 1);
    assert_eq!(VAR3.code, 2);
    assert_eq!(VAR4.foo, "VAR4");
    assert_eq!(VAR5.foo, "VAR5");
    assert_eq!(VAR6.foo, "VAR6");
    assert_eq!(VAR4.field_a, "yes");
    assert_eq!(VAR5.field_a, "no");
    assert_eq!(VAR6.field_a, "maybe");
    assert_eq!(VAR4.field_b, 17);
    assert_eq!(VAR5.field_b, 18);
    assert_eq!(VAR6.field_b, 19);
    assert_eq!(VAR7.foo, "VAR7");
    assert_eq!(VAR8.foo, "VAR8");
    assert_eq!(VAR7.field_c, 0);
    assert_eq!(VAR8.field_c, 1);
    assert_eq!(VAR8.field_a, "no");
    assert_eq!(
        VAR8.field_b,
        pubkey!("EULQ7RXBmMideABHPYz4ifk4cfNuuWNMBMAod8ZQxXFa")
    );
    assert_eq!(VAR9.field_c, 2);
}
