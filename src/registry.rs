pub struct ConfigRegistry<E, const N: usize> {
    pub version: u32,
    pub entries: [RegistryEntry<E>; N],
}

pub struct RegistryEntry<E> {
    pub key: &'static str,
    pub default_value: Option<&'static str>,
    pub entry_type: E,
}
