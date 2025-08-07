use figen::Configuration;


#[cfg(feature = "std")]
type String = std::string::String;
#[cfg(not(feature = "std"))]
type String = heapless::String<16>;

#[derive(Configuration)]
struct TestStruct {
    #[property(default = 1234)]
    field1: i32,

    #[property(default = "default_value")]
    field2: String,

    #[property(default = true)]
    field3: bool,

    #[property(default = 56)]
    field4: Option<i32>,

    #[property]
    field5: [Nested; 2],
}

#[derive(Configuration)]
struct Nested {
    #[property(default = 1234)]
    field1: u16,
}


#[test]
fn should_derive_default() {
    //
    // Given
    //
    let default = TestStruct::default();

    //
    // Expect
    //
    assert_eq!(default.field1, 1234);
    assert_eq!(default.field2.as_str(), "default_value");
    assert_eq!(default.field3, true);
    assert_eq!(default.field4, Some(56));
    assert_eq!(default.field5[0].field1, 1234);
    assert_eq!(default.field5[1].field1, 1234);
}