use crate::registry1::REGISTRY;
use figen::binder::ConfigBinder;
use figen::registry::Value::Number;
use figen::registry::Value::String;
use figen::BindPath;

mod utils;

#[derive(Default, Debug)]
struct CustomType {
    value: i32,
}

mod registry1 {
    use super::CustomType;
    use figen::{config_binder, config_registry};

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
}

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
    use registry1::*;
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
    use registry1::*;
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

mod registry2 {
    use figen::config_registry;

    config_registry!(
        version = 1

        num_property("field1", TestPath, default = 0)
        bool_property("field2.field.enabled", TestPath, optional, detaul = true)
        str_property("field2.field.aux", TestPath, optional, max_len = 8, default = "aux")
        num_property("field2.field.threshold", TestPath, optional, ty = u8)
        num_property("field3", TestPath, default = 30, ty = u16)
    );
}

#[test]
pub fn path_should_be_empty_on_ok() {
    use registry2::*;
    let mut path = figen::BindPathImpl::new();
    let loader = utils::MockLoader::new();

    let mut config = TestPathConfig::default();

    let result = config.bind(&mut path, &loader);
    match result {
        Ok(_) => {
            assert!(
                path.current_path().is_empty(),
                "Path should be empty after successful binding, but got: {}",
                path.current_path()
            );
        }
        Err(e) => {
            panic!(
                "Binding should have succeeded, but failed unexpectedly with error: {:?} at {}",
                e,
                path.current_path()
            );
        }
    }
}

#[test]
pub fn should_generate_registry() {
    use registry2::*;

    let reg = &REGISTRY;

    assert_eq!(reg.get_version(), 1, "Registry version should be 1");
    assert!(
        reg.has_entry("field1"),
        "Registry should have property 'field1'"
    );
    assert!(
        reg.has_entry("field2.field.enabled"),
        "Registry should have property 'field2.field.enabled'"
    );
    assert!(
        reg.has_entry("field2.field.aux"),
        "Registry should have property 'field2.field.aux'"
    );
    assert!(
        reg.has_entry("field2.field.threshold"),
        "Registry should have property 'field2.field.threshold'"
    );
    assert!(
        reg.has_entry("field3"),
        "Registry should have property 'field3'"
    );
    assert!(
        !reg.has_entry("non.existent.property"),
        "Registry should not have property 'non.existent.property'"
    );

    assert_eq!(reg.get_default_value("field1"), Some(Number(0)).as_ref());
    assert_eq!(
        reg.get_default_value("field2.field.aux"),
        Some(String("aux".into())).as_ref()
    );
    assert_eq!(reg.get_default_value("field2.field.threshold"), None);
    assert_eq!(reg.get_default_value("field3"), Some(Number(30)).as_ref());
}

#[test]
fn should_serialize_registry() {
    use registry2::*;
    #[cfg(feature = "std")]
    let config_registry = &*REGISTRY;
    #[cfg(not(feature = "std"))]
    let config_registry = &REGISTRY;


    let serialized = serde_json::to_string(config_registry).expect("Failed to serialize registry");

    assert!(serialized.contains("\"version\":1"));
    assert!(serialized.contains("\"key\":\"field1\""));
    assert!(serialized.contains("\"default_value\":{\"Number\":0}"));
    assert!(serialized.contains("\"key\":\"field2.field.enabled\""));
    assert!(serialized.contains("\"key\":\"field2.field.aux\""));
    assert!(serialized.contains("\"default_value\":{\"String\":\"aux\"}"));
    assert!(serialized.contains("\"key\":\"field2.field.threshold\""));
    assert!(serialized.contains("\"key\":\"field3\""));
}
