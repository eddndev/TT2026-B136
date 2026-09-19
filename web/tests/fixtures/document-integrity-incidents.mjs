export const incidentCaseId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
export const incidentDocumentId = '78ac67b1-ab36-49ea-9b08-f951f341f081';
export const incidentId = (value) => `10000000-0000-4000-8000-${String(value).padStart(12, '0')}`;
export function incidentRecord(value = 1, failure = 'digest_mismatch') {
  return {
    id: incidentId(value),
    observation_id: `20000000-0000-4000-8000-${String(value).padStart(12, '0')}`,
    case_id: incidentCaseId,
    document_id: incidentDocumentId,
    document_version: 1,
    requester_id: '30000000-0000-4000-8000-000000000001',
    failure,
    detected_at: '2026-09-19T10:00:00.123456788Z',
    recorded_at: '2026-09-19T10:00:00.123456789Z',
    expected_digest: 'a'.repeat(64),
    observed_snapshot_digest: 'b'.repeat(64),
  };
}
export function incidentPage(incidents = [], has_more = false) {
  return { incidents, has_more, next_after_id: has_more ? incidents.at(-1).id : null };
}
