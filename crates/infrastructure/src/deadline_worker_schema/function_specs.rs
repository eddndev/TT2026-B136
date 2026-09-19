pub(super) struct FunctionSpec {
    pub signature: &'static str,
    pub returns: &'static str,
    pub volatility: &'static str,
    pub types: &'static [&'static str],
    pub names: &'static [&'static str],
}

impl FunctionSpec {
    const fn trigger(signature: &'static str) -> Self {
        Self {
            signature,
            returns: "trigger",
            volatility: "v",
            types: &[],
            names: &[],
        }
    }
}

pub(super) const FUNCTIONS: &[FunctionSpec] = &[
    FunctionSpec {
        signature: "deadline_worker_cause(uuid)",
        returns: "jsonb",
        volatility: "v",
        types: &["uuid"],
        names: &["job"],
    },
    FunctionSpec {
        signature: "deadline_worker_authorized(uuid,uuid,uuid,jsonb)",
        returns: "uuid",
        volatility: "v",
        types: &["uuid", "uuid", "uuid", "jsonb"],
        names: &["scoped_case", "deadline", "operation", "receipt"],
    },
    FunctionSpec {
        signature: "deadline_worker_observations(case_deadline_revisions,bytea)",
        returns: "jsonb",
        volatility: "v",
        types: &["case_deadline_revisions", "bytea"],
        names: &["base", "bytes"],
    },
    FunctionSpec {
        signature: "deadline_worker_administration(case_deadline_revisions,bigint,bytea)",
        returns: "bytea",
        volatility: "v",
        types: &["case_deadline_revisions", "bigint", "bytea"],
        names: &["base", "revision", "evidence_digest"],
    },
    FunctionSpec {
        signature: "deadline_worker_review(case_deadline_revisions,jsonb,jsonb)",
        returns: "boolean",
        volatility: "v",
        types: &["case_deadline_revisions", "jsonb", "jsonb"],
        names: &["base", "entries", "tracking"],
    },
    FunctionSpec {
        signature: "deadline_worker_calendar_input(bytea,bytea,bigint)",
        returns: "boolean",
        volatility: "i",
        types: &["bytea", "bytea", "bigint"],
        names: &["before_bytes", "after_bytes", "revision"],
    },
    FunctionSpec {
        signature: "enforce_deadline_technical(case_deadline_revisions)",
        returns: "void",
        volatility: "v",
        types: &["case_deadline_revisions"],
        names: &["value"],
    },
    FunctionSpec::trigger("validate_deadline_worker_result()"),
    FunctionSpec::trigger("require_deadline_worker_result()"),
    FunctionSpec::trigger("validate_deadline_worker_attempt()"),
];

pub(super) fn signatures(callable: bool) -> Vec<&'static str> {
    FUNCTIONS
        .iter()
        .filter(|spec| (spec.returns != "trigger") == callable)
        .map(|spec| spec.signature)
        .collect()
}
