//! Strict reconstruction of typed values projected from canonical database bytes.

mod primitives;
mod profile;

use application::ApplicationError;
use domain::typed_participants::*;
use serde_json::Value;

use primitives::*;

type Result<T> = std::result::Result<T, ApplicationError>;

/// Rejects altered projections, unknown fields and implicit normalization.
pub fn subject(canonical: &[u8], projection: &Value) -> Result<SubjectValues> {
    if !(7..=5676).contains(&canonical.len()) || !canonical.starts_with(b"SUBJ1") {
        return Err(inconsistent());
    }
    let value = match string(&projection["kind"])? {
        "natural_person" => {
            fields(projection, &["kind", "name", "curp", "identity_support"])?;
            SubjectValues::natural_person(
                name(&projection["name"])?,
                declared(&projection["curp"], curp)?,
                support(&projection["identity_support"])?,
            )
        }
        "institutional_body" => {
            fields(
                projection,
                &[
                    "kind",
                    "name",
                    "institutional_identifier",
                    "identity_support",
                ],
            )?;
            SubjectValues::institutional_body(
                text(&projection["name"])?,
                declared(&projection["institutional_identifier"], text)?,
                support(&projection["identity_support"])?,
            )
        }
        _ => return Err(inconsistent()),
    };
    if value.canonical_bytes() != canonical {
        return Err(inconsistent());
    }
    Ok(value)
}

/// Reconstructs one exact subject reference and typed directory role snapshot.
pub fn participant(canonical: &[u8], projection: &Value) -> Result<TypedParticipantValues> {
    if !(62..=8380).contains(&canonical.len()) || !canonical.starts_with(b"PART2") {
        return Err(inconsistent());
    }
    fields(
        projection,
        &[
            "subject",
            "directory_status",
            "organization",
            "legal_status",
            "profile",
            "role_support",
        ],
    )?;
    let subject = &projection["subject"];
    fields(subject, &["id", "revision", "digest"])?;
    let subject = SubjectRevisionRef {
        id: CaseSubjectId::from_uuid(uuid(&subject["id"])?),
        revision: SubjectRevision::new(integer(&subject["revision"])?)
            .map_err(|_| inconsistent())?,
        values_digest: digest(&subject["digest"])?,
    };
    let directory_status = match string(&projection["directory_status"])? {
        "active" => DirectoryStatus::Active,
        "archived" => DirectoryStatus::Archived,
        _ => return Err(inconsistent()),
    };
    let role = ParticipantRoleValues::new(
        optional::<200>(&projection["organization"])?,
        optional::<160>(&projection["legal_status"])?,
        profile::parse(&projection["profile"])?,
        support(&projection["role_support"])?,
    )
    .map_err(|_| inconsistent())?;
    let value = TypedParticipantValues::new(subject, directory_status, role);
    if value.canonical_bytes() != canonical {
        return Err(inconsistent());
    }
    Ok(value)
}

fn name(value: &Value) -> Result<RepresentedName> {
    if value.get("known").is_some() {
        fields(value, &["known"])?;
        Ok(RepresentedName::Known(text(&value["known"])?))
    } else {
        fields(value, &["label", "reason"])?;
        Ok(RepresentedName::Unidentified {
            label: text(&value["label"])?,
            reason: reason(&value["reason"])?,
        })
    }
}

fn inconsistent() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "stored typed participant values are inconsistent".into(),
    )
}
