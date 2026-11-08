import test from 'node:test';
import assert from 'node:assert/strict';
import { invitationsApi } from '../src/lib/invitations-api.mjs';

const id = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const row = (role) => ({
  id,
  operation_id: id,
  revision: '1',
  email: 'client@example.test',
  role,
  status: 'pending',
  delivery: 'pending',
  issued_at: '2026-11-02T00:00:00Z',
  expires_at: '2026-11-03T00:00:00Z',
  accepted_at: null,
});

test('an owner may invite and list a client account but never an owner', async () => {
  const calls = [];
  const api = invitationsApi(async (path, options) => {
    calls.push([path, options]);
    return path.startsWith('/member-invitations?')
      ? { items: [row('client')], next: null }
      : row('client');
  });
  const issued = await api.issue({
    operation_id: id,
    email: 'client@example.test',
    name: 'Cliente Ejemplo',
    role: 'client',
  });
  assert.equal(issued.role, 'client');
  assert.equal((await api.list()).items[0].role, 'client');
  await assert.rejects(
    api.issue({ operation_id: id, email: 'owner@example.test', name: 'Ana', role: 'owner' }),
  );
  for (const name of [undefined, '', ' Ana', 'Ana\u202e', 7])
    await assert.rejects(
      api.issue({ operation_id: id, email: 'client@example.test', name, role: 'client' }),
    );
  assert.equal(calls.length, 2);
  assert.equal(calls[0][1].data.name, 'Cliente Ejemplo');
  const owner = invitationsApi(async () => row('owner'));
  await assert.rejects(owner.get(id));
});
