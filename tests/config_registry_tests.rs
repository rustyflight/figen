use figen::expand_config_registry; 

expand_config_registry!(
    version = 1

    str_property("str_prop", Group1, default = "abc", max_len = 4)
    num_property("num_prop", Group1, default = 42)
    num_property("array_prop[0]", Group1, default = 1)
    num_property("array_prop[1]", Group1, default = 2)
    str_property("optional_str_prop", Group1, max_len = 4, optional)
    num_property("deeply.nested.prop", Group1, default = 100)
    str_property("deeply.nested.prop2", Group1, default = "def", max_len = 3)
);

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
}
