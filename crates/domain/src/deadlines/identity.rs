use super::DeadlineValueError;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);
        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }
            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}
identifier!(DeadlineId);
identifier!(DeadlineOperationId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32")]
pub struct DeadlineRevision(u32);
impl DeadlineRevision {
    pub const fn initial() -> Self {
        Self(1)
    }
    pub fn new(value: u32) -> Result<Self, DeadlineValueError> {
        if value == 0 {
            Err(DeadlineValueError::InvalidRevision)
        } else {
            Ok(Self(value))
        }
    }
    pub const fn get(self) -> u32 {
        self.0
    }
    pub fn next(self) -> Result<Self, DeadlineValueError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(DeadlineValueError::RevisionExhausted)
    }
}
impl TryFrom<u32> for DeadlineRevision {
    type Error = DeadlineValueError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
