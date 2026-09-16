use super::*;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectQuery {
    limit: u32,
    after_id: Option<CaseSubjectId>,
    name: Option<String>,
    kind: Option<SubjectKind>,
}
impl SubjectQuery {
    pub fn new(
        limit: u32,
        after_id: Option<CaseSubjectId>,
        name: Option<&str>,
        kind: Option<SubjectKind>,
    ) -> Result<Self, ApplicationError> {
        validate_limit(limit)?;
        let name = match name {
            Some(v) if v.chars().any(char::is_control) => {
                return Err(ApplicationError::InvalidInput(
                    "subject name filter contains controls".into(),
                ))
            }
            Some(v) if !v.trim().is_empty() => {
                Some(ParticipantText::<200>::new(v)?.as_str().to_owned())
            }
            _ => None,
        };
        Ok(Self {
            limit,
            after_id,
            name,
            kind,
        })
    }
    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn after_id(&self) -> Option<CaseSubjectId> {
        self.after_id
    }
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    pub const fn kind(&self) -> Option<SubjectKind> {
        self.kind
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectHistoryQuery {
    limit: u32,
    before_revision: Option<SubjectRevision>,
}
impl SubjectHistoryQuery {
    pub fn new(
        limit: u32,
        before_revision: Option<SubjectRevision>,
    ) -> Result<Self, ApplicationError> {
        validate_limit(limit)?;
        Ok(Self {
            limit,
            before_revision,
        })
    }
    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn before_revision(&self) -> Option<SubjectRevision> {
        self.before_revision
    }
}
fn validate_limit(limit: u32) -> Result<(), ApplicationError> {
    if !(1..=100).contains(&limit) {
        return Err(ApplicationError::InvalidInput(
            "subject query limit must be between 1 and 100".into(),
        ));
    }
    Ok(())
}
