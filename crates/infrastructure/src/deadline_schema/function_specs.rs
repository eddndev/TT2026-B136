//! Exact function signatures and execution contracts protected at startup.
pub(super) struct FunctionSpec {
    pub signature: &'static str,
    pub returns: &'static str,
    pub volatility: &'static str,
    pub callable: bool,
}

impl FunctionSpec {
    const fn parser(signature: &'static str, returns: &'static str) -> Self {
        Self {
            signature,
            returns,
            volatility: "i",
            callable: true,
        }
    }
    const fn trigger(signature: &'static str) -> Self {
        Self {
            signature,
            returns: "trigger",
            volatility: "v",
            callable: false,
        }
    }
}

pub(super) const FUNCTIONS: &[FunctionSpec] = &[
    FunctionSpec::parser("deadline_submission(bytea)", "jsonb"),
    FunctionSpec::parser("deadline_attention_valid(jsonb)", "boolean"),
    FunctionSpec::parser("deadline_input_selection(bytea)", "jsonb"),
    FunctionSpec::parser("deadline_submission_v2(bytea)", "jsonb"),
    FunctionSpec::parser("deadline_observations(bytea)", "jsonb"),
    FunctionSpec::parser("deadline_tracking(bytea)", "jsonb"),
    FunctionSpec::parser("deadline_tracking_consistent(bytea,bytea)", "boolean"),
    FunctionSpec::trigger("preserve_deadline_history()"),
    FunctionSpec::trigger("enforce_deadline_sequence()"),
];

pub(super) fn signatures(callable: bool) -> Vec<&'static str> {
    FUNCTIONS
        .iter()
        .filter(|spec| spec.callable == callable)
        .map(|spec| spec.signature)
        .collect()
}
