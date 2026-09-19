// Alert occurrences keep the exact origin captured when they were activated.
export function hearingOccurrenceMatches(row, scenario) {
  return (
    row.subject.kind === 'hearing' &&
    row.subject.case_id === scenario.case.id &&
    row.subject.id === scenario.hearing.id &&
    row.state.kind === 'active' &&
    row.kind.kind === 'upcoming' &&
    row.kind.lead_hours === 48 &&
    [scenario.hearingInitial, scenario.hearing].some(
      (snapshot) =>
        row.origin.revision === snapshot.revision &&
        row.origin.evidence_digest === snapshot.receipt.submission_digest,
    )
  );
}
