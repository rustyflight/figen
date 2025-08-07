use figen::Configuration;
use figen::error::Error;

mod utils;

#[derive(Configuration, Debug)]
struct ArrayRefConfig {
    #[property(array_ref(key = "my_array", prefix = "NESTED_"))]
    array_ref: Nested,
    #[property(array_ref(key = "my_array", prefix = "NESTED_"))]
    optional_array_ref: Option<Nested>
}

#[derive(Configuration, Debug)]
struct Nested {
    #[property(default = 20)]
    field1: i32,
    #[property(default = 30)]
    field2: i32
}

#[test]
fn should_load_array_ref() {
    let loader = utils::MockLoader::new()
        .with_data("my_array.0.field1", "1")
        .with_data("my_array.0.field2", "2")
        .with_data("array_ref", "NESTED_0");

    let config  = figen::load_config::<ArrayRefConfig, utils::MockLoader, utils::BindPathImpl>(&loader).unwrap();

    assert_eq!(config.array_ref.field1, 1);
    assert_eq!(config.array_ref.field2, 2);
}

#[test]
fn should_err_not_found() {
    let loader = utils::MockLoader::new()
        .with_data("my_array.0.field1", "1")
        .with_data("my_array.0.field2", "2")
        .with_data("array_ref", "NESTED_1"); // Non Existing index

    let config  = figen::load_config::<ArrayRefConfig, utils::MockLoader, utils::BindPathImpl>(&loader);
    assert!(config.is_err(), "Expected an error when trying to load a non-existing array reference");
    let err = config.unwrap_err();
    assert_eq!(err, Error::Required, "Expected Required error, got {:?}", err);
}

#[test]
fn should_return_none_for_optional_ref() {
    let loader = utils::MockLoader::new()
        .with_data("my_array.0.field1", "1")
        .with_data("my_array.0.field2", "2")
        .with_data("optional_array_ref", "NESTED_1"); // Non Existing index

    let config  = figen::load_config::<ArrayRefConfig, utils::MockLoader, utils::BindPathImpl>(&loader);
    assert!(config.is_ok(), "Expected an Ok when trying to load a non-existing array reference");
    let config = config.unwrap();
    assert!(config.optional_array_ref.is_none(), "Expected optional_array_ref to be None, got {:?}", config.optional_array_ref);
}
