use solana_devtools_macros::const_data;

#[const_data(
    "VAR1", "foo", 84u64;
    "VAR2", "bar", 87u64;
)]
struct TypeWithCode {
    field_a: &'static str,
    field_b: u64,
    #[code]
    code: u32,
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
    assert_eq!(type_without_code::NUM_CONSTS, 2usize);
}
