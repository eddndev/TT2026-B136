export const memberId = (n) => `10000000-0000-4000-8000-${String(n).padStart(12, '0')}`;
export const memberCaseId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
export const otherMemberCaseId = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
export const memberRecord = (n = 2, values = {}) => ({
  id: memberId(n),
  email: `person${n}@example.test`,
  role: 'paralegal',
  active: true,
  revision: '9007199254740993',
  ...values,
});
export const memberActor = memberRecord(1, {
  email: 'owner@example.test',
  role: 'owner',
  revision: '4',
});
export const memberPage = (items = [], next = null) => ({
  items,
  has_more: next !== null,
  next_cursor: next,
});
export const assignedMember = (n = 2, values = {}) => ({
  ...memberRecord(n, values),
  assigned_at: '2026-09-19T10:00:00.123456789Z',
});
export const caseMemberPage = (items = [], next = null, caseId = memberCaseId) => ({
  case_id: caseId,
  ...memberPage(items, next),
});
