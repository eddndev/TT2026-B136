import { memberQuery } from './members-query.mjs';
import {
  memberUuid,
  memberSummary,
  memberPageValue,
  memberAccessChange,
  memberAccessResult,
} from './members-values.mjs';

function memberScope(request) {
  let active = true;
  const assertActive = () => {
    if (!active) throw new Error('La consulta de cuentas ya no est\u00e1 abierta.');
  };
  return {
    assertActive,
    dispose() {
      active = false;
    },
    async call(path, options) {
      assertActive();
      try {
        const value = await request(path, options);
        assertActive();
        return value;
      } catch (failure) {
        assertActive();
        throw failure;
      }
    },
  };
}
export function membersApi(request, onAccessChanged = () => {}) {
  const scope = memberScope(request);
  return {
    dispose: scope.dispose,
    async list(input) {
      scope.assertActive();
      const query = memberQuery(input);
      return memberPageValue(await scope.call(`/users?${new URLSearchParams(query)}`), query);
    },
    async get(id) {
      memberUuid(id);
      return memberSummary(await scope.call(`/users/${id}`), id);
    },
    async changeAccess(id, input, onConfirmed = () => {}) {
      if (typeof onConfirmed !== 'function')
        throw new TypeError('A confirmation callback is required.');
      memberUuid(id);
      const change = memberAccessChange(input);
      const result = memberAccessResult(
        await scope.call(`/users/${id}/access`, {
          method: 'PUT',
          data: change,
        }),
        id,
        change,
      );
      try {
        onConfirmed(result);
      } finally {
        if (result.revision !== change.expected_revision) onAccessChanged(result);
      }
      return result;
    },
  };
}
export function caseMembersApi(request, caseId) {
  memberUuid(caseId);
  const scope = memberScope(request),
    base = `/cases/${caseId}/members`;
  const mutate = async (id, method) => {
    memberUuid(id);
    const value = await scope.call(`${base}/${id}`, { method });
    if (value !== null) throw new Error('La respuesta de asignaciones no es v\u00e1lida.');
    return value;
  };
  return {
    dispose: scope.dispose,
    async list(input) {
      scope.assertActive();
      const query = memberQuery(input, true);
      return memberPageValue(
        await scope.call(`${base}?${new URLSearchParams(query)}`),
        query,
        caseId,
      );
    },
    assign: (id) => mutate(id, 'PUT'),
    remove: (id) => mutate(id, 'DELETE'),
  };
}
