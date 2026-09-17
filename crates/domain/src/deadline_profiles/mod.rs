//! Explicit quantity binding for arithmetic rules, without legal qualification.

mod rules;

pub use rules::{DeadlineRuleBlock, DeadlineRuleTemplate, OrderedDeadlineUnit};

mod catalog;
mod identity;
pub use catalog::*;
pub use identity::*;
