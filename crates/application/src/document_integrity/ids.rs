use domain::typed_participants::Uuid;
use std::fmt;

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}
identifier!(DocumentIntegrityObservationId);
identifier!(DocumentIntegrityIncidentId);
