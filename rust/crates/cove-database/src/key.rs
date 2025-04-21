pub mod outpoint;

use std::fmt;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DatabaseKey<T>(pub T);

impl<T: AsRef<[u8]>> AsRef<[u8]> for DatabaseKey<T> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<T: fmt::Debug> fmt::Debug for DatabaseKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DatabaseKey").field(&self.0).finish()
    }
}

pub trait IntoBytes {
    fn into_bytes(self) -> Vec<u8>;
}

pub trait FromBytes: Sized {
    fn from_bytes(bytes: &[u8]) -> Option<Self>;
}