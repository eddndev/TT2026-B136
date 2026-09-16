use super::{helpers::*, inconsistent, Result};
use application::procedural_facts::*;
use domain::participants::{ParticipantId, ParticipantRevision};
use serde_json::Value;

pub(super) fn declaration<T>(
    value: &Value,
    parse: impl FnOnce(&Value) -> Result<T>,
) -> Result<FactDeclaration<T>> {
    match string(&value["kind"])? {
        "unknown" => {
            fields(value, &["kind", "reason"])?;
            Ok(FactDeclaration::Unknown(text(&value["reason"])?))
        }
        "known" => {
            fields(value, &["kind", "value"])?;
            Ok(FactDeclaration::Known(parse(&value["value"])?))
        }
        _ => Err(inconsistent()),
    }
}
pub(super) fn source_declaration<T>(
    value: &Value,
    parse: impl FnOnce(&Value) -> Result<T>,
) -> Result<FactDeclaration<T>> {
    if value.get("unknown").is_some() {
        fields(value, &["unknown"])?;
        Ok(FactDeclaration::Unknown(text(&value["unknown"])?))
    } else {
        fields(value, &["known"])?;
        Ok(FactDeclaration::Known(parse(&value["known"])?))
    }
}
macro_rules! catalog {
    ($function:ident, $kind:ident, $first:literal => $a:ident, $second:literal => $b:ident) => {
        pub(super) fn $function(value: &Value) -> Result<$kind> {
            let kind = string(&value["kind"])?;
            match kind {
                $first | $second => fields(value, &["kind"])?,
                "other" => fields(value, &["kind", "label"])?,
                _ => return Err(inconsistent()),
            }
            match kind {
                $first => Ok($kind::$a),
                $second => Ok($kind::$b),
                "other" => Ok($kind::Other(label(&value["label"])?)),
                _ => Err(inconsistent()),
            }
        }
    };
}
catalog!(class, ResolutionClass, "order" => Order, "judgment" => Judgment);
catalog!(character, NotificationCharacter, "personal" => Personal, "publication" => Publication);
catalog!(medium, NotificationMedium, "in_person" => InPerson, "electronic" => Electronic);
catalog!(context, NotificationContext, "in_hearing" => InHearing, "outside_hearing" => OutsideHearing);
pub(super) fn outcome(value: &Value) -> Result<NotificationOutcome> {
    fields(value, &["kind"])?;
    match string(&value["kind"])? {
        "practiced" => Ok(NotificationOutcome::Practiced),
        "attempted" => Ok(NotificationOutcome::Attempted),
        _ => Err(inconsistent()),
    }
}
pub(super) fn person(value: &Value) -> Result<FactPerson> {
    match string(&value["kind"])? {
        "participant" => {
            fields(value, &["kind", "id", "revision"])?;
            Ok(FactPerson::Participant(FactParticipantRef {
                id: ParticipantId::from_uuid(uuid(&value["id"])?),
                revision: ParticipantRevision::new(counter(&value["revision"])?)
                    .map_err(|_| inconsistent())?,
            }))
        }
        "unlinked" => {
            fields(value, &["kind", "label", "description"])?;
            Ok(FactPerson::Unlinked {
                label: label(&value["label"])?,
                description: text(&value["description"])?,
            })
        }
        _ => Err(inconsistent()),
    }
}
