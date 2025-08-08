use figen::Configuration;

mod utils;

#[derive(Configuration, Debug, Default)]
struct MyConfig {
    #[property(optional)]
    i32_field: i32,
    #[property(optional)]
    bool_field: bool,
    #[property(optional)]
    string_field: utils::StringType,
    #[property(optional)]
    optional_field: Option<i32>,
    #[property]
    nested_array: [i32; 2],
    #[property(indices = ["i1", "i2"])]
    custom_indexed: [i32; 2],
}

#[test]
fn should_attempt_to_load_keys() {
    let loader = utils::MockLoader::new();

    let _: figen::Result<MyConfig> = figen::load_config(&loader);
    let attempted_keys = loader.get_attempted_keys();

    let expected_keys = vec![
        "i32_field".to_string(),
        "bool_field".to_string(),
        "string_field".to_string(),
        "optional_field".to_string(),
        "nested_array[0]".to_string(),
        "nested_array[1]".to_string(),
        "custom_indexed[i1]".to_string(),
        "custom_indexed[i2]".to_string(),
    ];
    assert_eq!(attempted_keys, expected_keys, "The keys attempted to load do not match the expected keys.");
}

#[test]
fn should_load_config_values() {
    let loader = utils::MockLoader::new()
        .with_data("i32_field", "1234")
        .with_data("bool_field", "false") // Different from default
        .with_data("string_field", "test_string")
        .with_data("optional_field", "12") // Different from default
        .with_data("nested_array[0]", "1")
        .with_data("nested_array[1]", "2")
        .with_data("custom_indexed[i1]", "30")
        .with_data("custom_indexed[i2]", "40");

    let config: MyConfig = figen::load_config(&loader).expect("Failed to load configuration");

    assert_eq!(config.i32_field, 1234);
    assert_eq!(config.bool_field, false);
    assert_eq!(config.string_field, "test_string");
    assert_eq!(config.optional_field, Some(12));
    assert_eq!(config.nested_array[0], 1);
    assert_eq!(config.nested_array[1], 2);
    assert_eq!(config.custom_indexed[0], 30);
    assert_eq!(config.custom_indexed[1], 40);
}


#[derive(Configuration, Debug, Default)]
struct TestConfig {
    #[property] // no default value
    required_field: i32,
    #[property()]
    optional_field: i32,
    #[property] // Option<> types are fine with no default
    optional_field2: Option<i32>,
}

#[test]
fn should_err_when_required_property_missing() {
    let loader = utils::MockLoader::new()
        .with_data("optional_field", "200")
        .with_data("optional_field2", "300");

    let result: figen::Result<TestConfig> = figen::load_config(&loader);

    assert!(result.is_err(), "Expected an error when required property is missing and no default provided");
    let err = result.unwrap_err();
    assert_eq!(err, figen::error::Error::Required, "Expected Required error, got {:?}", err);
}
