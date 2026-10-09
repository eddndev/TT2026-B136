use super::*;
use domain::{cases::CaseId, identity::UserId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) enum StoredKind {
    #[serde(rename = "litigator_activity")]
    Activity,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Actor {
    id: Uuid,
    email: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    case_id: Uuid,
    actor_id: Uuid,
    documents_uploaded: u64,
    procedural_activities: u64,
    deadlines_attended: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Activity {
    actors: Vec<Actor>,
    rows: Vec<Row>,
    documents_complete: bool,
}
impl Activity {
    pub(super) fn write(value: &CaseReportActivitySnapshot) -> Self {
        Self {
            actors: value
                .actors
                .iter()
                .map(|who| Actor {
                    id: who.user_id.as_uuid(),
                    email: who.email.clone(),
                })
                .collect(),
            rows: value
                .rows
                .iter()
                .map(|row| Row {
                    case_id: row.case_id.as_uuid(),
                    actor_id: row.litigator_id.as_uuid(),
                    documents_uploaded: row.documents_uploaded,
                    procedural_activities: row.procedural_activities,
                    deadlines_attended: row.deadlines_attended,
                })
                .collect(),
            documents_complete: value.documents_complete,
        }
    }
    pub(super) fn read(self) -> CaseReportActivitySnapshot {
        CaseReportActivitySnapshot {
            actors: self
                .actors
                .into_iter()
                .map(|who| CaseReportLitigator {
                    user_id: UserId::from_uuid(who.id),
                    email: who.email,
                })
                .collect(),
            rows: self
                .rows
                .into_iter()
                .map(|row| CaseReportActivityRow {
                    case_id: CaseId::from_uuid(row.case_id),
                    litigator_id: UserId::from_uuid(row.actor_id),
                    documents_uploaded: row.documents_uploaded,
                    procedural_activities: row.procedural_activities,
                    deadlines_attended: row.deadlines_attended,
                })
                .collect(),
            documents_complete: self.documents_complete,
        }
    }
}
