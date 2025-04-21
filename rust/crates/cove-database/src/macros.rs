// Database macros for common operations

#[macro_export]
macro_rules! define_table {
    ($name:ident, $key_type:ty, $value_type:ty) => {
        pub static $name: once_cell::sync::Lazy<redb::TableDefinition<$key_type, $value_type>> =
            once_cell::sync::Lazy::new(|| redb::TableDefinition::new(stringify!($name)));
    };
}

#[macro_export]
macro_rules! define_multi_map_table {
    ($name:ident, $key_type:ty, $value_type:ty) => {
        pub static $name: once_cell::sync::Lazy<redb::MultiMapTableDefinition<$key_type, $value_type>> =
            once_cell::sync::Lazy::new(|| redb::MultiMapTableDefinition::new(stringify!($name)));
    };
}