use super::{FactLabel, FactText};

/// Declared class, without deriving availability of a remedy or legal effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionClass {
    Order,
    Judgment,
    Other(FactLabel),
}

/// Character expressly attributed to a notification practice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationCharacter {
    Personal,
    Publication,
    Other(FactLabel),
}

/// Medium is independent of the declared character and context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationMedium {
    InPerson,
    Electronic,
    Other(FactLabel),
}

/// Context does not infer attendance, receipt or notification effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationContext {
    InHearing,
    OutsideHearing,
    Other(FactLabel),
}

/// Declared outcome; neither variant determines legal effectiveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationOutcome {
    Practiced,
    Attempted,
}

/// Unknown information retains its explicit explanation instead of a default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactDeclaration<T> {
    Known(T),
    Unknown(FactText),
}
