use super::{incomplete, port, MIGRATIONS, TABLE};
use application::ApplicationError;
use postgres::GenericClient;

pub(super) const KEYS: &[(&str, &str, bool)] = &[
    ("password_reset_capabilities_pkey", "id", true),
    ("password_reset_capabilities_digest_key", "digest", false),
    (
        "password_reset_capabilities_audit_sequence_key",
        "audit_sequence",
        false,
    ),
];
type ForeignKey = (&'static str, &'static str, &'static str, &'static str, bool);
pub(super) const FOREIGN: &[ForeignKey] = &[
    (
        "password_reset_capabilities_user_id_fkey",
        "user_id",
        "users",
        "id",
        false,
    ),
    (
        "password_reset_capabilities_audit_sequence_fkey",
        "audit_sequence",
        "audit_events",
        "sequence",
        true,
    ),
];
const CHECKS: &[&str] = &[
    "password_reset_id_check",
    "password_reset_digest_check",
    "password_reset_generation_check",
    "password_reset_time_check",
    "password_reset_cancel_check",
    "password_reset_consumption_check",
];

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    for &(name, column, primary) in KEYS {
        let kind = if primary { "p" } else { "u" };
        let valid: bool = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_constraint c JOIN pg_class t ON t.oid=c.conrelid
            JOIN pg_class i ON i.oid=c.conindid WHERE c.conrelid=$1::text::regclass
                AND c.conname=$2 AND c.contype::text=$3 AND c.convalidated
                AND NOT c.condeferrable AND NOT c.condeferred AND c.conislocal
                AND c.coninhcount=0 AND c.conparentid=0 AND c.connamespace=t.relnamespace
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
                AND i.relname=c.conname AND i.relnamespace=t.relnamespace
                AND c.conkey=ARRAY[(SELECT attnum FROM pg_attribute WHERE attrelid=c.conrelid AND attname=$4)]::smallint[])",
            &[&TABLE, &name, &kind, &column],
        ).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for &(name, column, target, target_column, deferred) in FOREIGN {
        let valid: bool = client.query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_constraint c JOIN pg_class t ON t.oid=c.conrelid
            WHERE c.conrelid=$1::text::regclass AND c.conname=$2 AND c.contype='f'
                AND c.confrelid=$4::text::regclass AND c.convalidated
                AND c.condeferrable=$6 AND c.condeferred=$6 AND c.conislocal
                AND c.coninhcount=0 AND c.conparentid=0 AND c.connamespace=t.relnamespace
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
                AND c.confupdtype='a' AND c.confdeltype='a' AND c.confmatchtype='s'
                AND c.conkey=ARRAY[(SELECT attnum FROM pg_attribute WHERE attrelid=c.conrelid AND attname=$3)]::smallint[]
                AND c.confkey=ARRAY[(SELECT attnum FROM pg_attribute WHERE attrelid=c.confrelid AND attname=$5)]::smallint[])",
            &[&TABLE, &name, &column, &target, &target_column, &deferred],
        ).map_err(port)?.get(0);
        if !valid {
            return Err(incomplete());
        }
    }
    for name in CHECKS {
        let row = client
            .query_opt(
                "SELECT pg_get_expr(c.conbin,c.conrelid),c.convalidated AND NOT c.connoinherit
                AND c.conislocal AND c.coninhcount=0 AND c.conparentid=0
                AND c.connamespace=t.relnamespace AND NOT c.condeferrable AND NOT c.condeferred
                AND (to_jsonb(c)->>'conenforced') IS DISTINCT FROM 'false'
            FROM pg_constraint c JOIN pg_class t ON t.oid=c.conrelid
            WHERE c.conrelid=$1::text::regclass AND c.conname=$2 AND c.contype='c'",
                &[&TABLE, name],
            )
            .map_err(port)?
            .ok_or_else(incomplete)?;
        if !row.get::<_, bool>(1)
            || normalize(&row.get::<_, String>(0))
                != normalize(check_body(name).ok_or_else(incomplete)?)
        {
            return Err(incomplete());
        }
    }
    let count: i64 = client
        .query_one(
            "SELECT count(*) FROM pg_constraint WHERE conrelid=$1::text::regclass AND contype<>'n'",
            &[&TABLE],
        )
        .map_err(port)?
        .get(0);
    // The deferred receipt is represented by one additional trigger constraint.
    if count != (KEYS.len() + FOREIGN.len() + CHECKS.len() + 1) as i64 {
        return Err(incomplete());
    }
    Ok(())
}

fn check_body(name: &str) -> Option<&'static str> {
    let marker = format!("CONSTRAINT {name} CHECK (");
    let rest = MIGRATIONS[0].split_once(&marker)?.1;
    let mut depth = 1;
    for (offset, character) in rest.char_indices() {
        if character == '(' {
            depth += 1;
        }
        if character == ')' {
            depth -= 1;
        }
        if depth == 0 {
            return Some(&rest[..offset]);
        }
    }
    None
}

fn normalize(value: &str) -> String {
    let mut value = value.trim();
    while outer_group(value) {
        value = value[1..value.len() - 1].trim();
    }
    // Preserve boolean grouping: deleting every parenthesis can accept a weaker CHECK.
    for operator in ["or", "and"] {
        let parts = split_boolean(value, operator);
        if parts.len() > 1 {
            return format!(
                "{operator}[{}]",
                parts
                    .into_iter()
                    .map(normalize)
                    .collect::<Vec<_>>()
                    .join(";")
            );
        }
    }
    value
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && !matches!(c, '(' | ')' | '\'' | '"'))
        .flat_map(char::to_lowercase)
        .collect()
}

fn outer_group(value: &str) -> bool {
    if !value.starts_with('(') || !value.ends_with(')') {
        return false;
    }
    let mut depth = 0;
    let mut quoted = false;
    for (offset, byte) in value.bytes().enumerate() {
        if byte == b'\'' {
            quoted = !quoted;
        }
        if quoted {
            continue;
        }
        if byte == b'(' {
            depth += 1;
        }
        if byte == b')' {
            depth -= 1;
        }
        if depth == 0 && offset + 1 < value.len() {
            return false;
        }
    }
    depth == 0
}

fn split_boolean<'a>(value: &'a str, operator: &str) -> Vec<&'a str> {
    let bytes = value.as_bytes();
    let mut result = Vec::new();
    let (mut depth, mut quoted, mut start) = (0, false, 0);
    let word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    for (offset, byte) in bytes.iter().copied().enumerate() {
        if byte == b'\'' {
            quoted = !quoted;
        }
        if quoted {
            continue;
        }
        if byte == b'(' {
            depth += 1;
        }
        if byte == b')' {
            depth -= 1;
        }
        let end = offset + operator.len();
        if depth == 0
            && end <= bytes.len()
            && (offset == 0 || !word(bytes[offset - 1]))
            && (end == bytes.len() || !word(bytes[end]))
            && bytes[offset..end].eq_ignore_ascii_case(operator.as_bytes())
        {
            result.push(&value[start..offset]);
            start = end;
        }
    }
    result.push(&value[start..]);
    result
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn check_comparison_preserves_boolean_grouping() {
        assert_eq!(
            normalize("(a IS NULL OR (b>0 AND c>0))"),
            normalize("((a IS NULL) OR ((b > 0) AND (c > 0)))")
        );
        assert_ne!(
            normalize("a IS NULL OR (b>0 AND c>0)"),
            normalize("(a IS NULL OR b>0) AND c>0")
        );
    }
}
