use figen::{config_registry, config_binder};

mod utils;

#[derive(Default, Debug)]
struct CustomType {
    value: i32,
}

config_registry!(
    version = 1

    str_property("str_prop", Group1, default = "abc", max_len = 4)
    num_property("num_prop", Group1, default = 42)
    num_property("array_prop[0]", Group1, default = 1)
    num_property("array_prop[1]", Group1, default = 2)
    num_property("array_prop[custom]", Group1, default = 2)
    str_property("optional_str_prop", Group1, max_len = 4, optional)
    num_property("deeply.nested.prop", Group1, default = 100)
    str_property("deeply.nested.prop2", Group1, default = "def", max_len = 3)
    custom_property("custom_prop", Group1, default = "12", ty = CustomType)
);

config_binder!(CustomType);

impl TryFrom<&str> for CustomType {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(CustomType {
            value: value.parse().map_err(|_| "Failed to parse CustomType")?,
        })
    }
}

#[test]
pub fn test_generated_fields() {
    let group1_config = Group1Config::default();

    assert_eq!(group1_config.str_prop, "abc");
    assert_eq!(group1_config.num_prop, 42);
    assert_eq!(group1_config.array_prop[0], 1);
    assert_eq!(group1_config.array_prop[1], 2);
    assert_eq!(group1_config.optional_str_prop, None);
    assert_eq!(group1_config.deeply.nested.prop, 100);
    assert_eq!(group1_config.deeply.nested.prop2, "def");
    assert_eq!(group1_config.custom_prop.value, 12);
}

#[test]
pub fn test_config_binding() {
    let loader = utils::MockLoader::new()
        .with_data("str_prop", "xyz")
        .with_data("num_prop", "99")
        .with_data("array_prop[0]", "3")
        .with_data("array_prop[1]", "4")
        .with_data("array_prop[custom]", "6")
        .with_data("deeply.nested.prop", "200")
        .with_data("deeply.nested.prop2", "ghi");

    let config: Group1Config = figen::load_config(&loader).expect("Failed to load configuration");

    assert_eq!(config.str_prop, "xyz");
    assert_eq!(config.num_prop, 99);
    assert_eq!(config.array_prop[0], 3);
    assert_eq!(config.array_prop[1], 4);
    assert_eq!(config.array_prop[2], 6);
    assert_eq!(config.optional_str_prop, None); // Optional property not set
    assert_eq!(config.deeply.nested.prop, 200);
    assert_eq!(config.deeply.nested.prop2, "ghi");
}
