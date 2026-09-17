use super::DeadlineProfileValueError;
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
identifier!(DeadlineProfileId);
identifier!(DeadlineProfileOperationId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32")]
pub struct DeadlineProfileRevision(u32);
impl DeadlineProfileRevision {
    pub const fn initial() -> Self {
        Self(1)
    }
    pub fn new(value: u32) -> Result<Self, DeadlineProfileValueError> {
        if value == 0 {
            Err(DeadlineProfileValueError::InvalidRevision)
        } else {
            Ok(Self(value))
        }
    }
    pub const fn get(self) -> u32 {
        self.0
    }
    pub fn next(self) -> Result<Self, DeadlineProfileValueError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(DeadlineProfileValueError::RevisionExhausted)
    }
}
impl TryFrom<u32> for DeadlineProfileRevision {
    type Error = DeadlineProfileValueError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
