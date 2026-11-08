import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  localOffset,
  dashboardAgendaSelection,
  dashboardAgendaRows,
  dashboardAgendaDay,
  dashboardAgendaLoader,
} from '../src/components/dashboard-agenda.mjs';

// 1767373200 is 2026-01-02T17:00:00Z, 11:00 of 2026-01-02 at -06:00.
const at = { unix_seconds: 1767373200, nanosecond: 0, offset_seconds: 0 };
const hearing = (id, unix = at.unix_seconds) => ({
  kind: 'hearing',
  at: { ...at, unix_seconds: unix },
  hearing: {
    id,
    revision: 1,
    kind: 'initial',
    status: 'scheduled',
    scheduled_at: new Date(unix * 1000).toISOString().replace('.000Z', 'Z'),
    case_title: 'Defensa inicial',
    case_reference: 'RTA-2026-014',
    case_status: 'active',
  },
});
const deadline = {
  kind: 'deadline',
  at,
  case_title: 'Amparo directo',
  case_reference: 'NUC-123',
  case_status: 'active',
  deadline: {
    id: 'd-1',
    revision: 1,
    title: 'Contestar demanda',
    responsible: { email: 'responsable@example.test' },
    attention_recorded: false,
  },
};

test('the local offset is explicit and never -00:00', () => {
  assert.equal(localOffset({ getTimezoneOffset: () => 360 }), '-06:00');
  assert.equal(localOffset({ getTimezoneOffset: () => -330 }), '+05:30');
  assert.equal(localOffset({ getTimezoneOffset: () => 0 }), '+00:00');
});

test('the coming agenda queries thirty local days of every kind', () => {
  const { range, query } = dashboardAgendaSelection('2026-01-01', '-06:00');
  assert.deepEqual(range, { from: '2026-01-01', until: '2026-01-31' });
  assert.deepEqual(query, {
    from: '2026-01-01T06:00:00Z',
    until: '2026-01-31T06:00:00Z',
    kind: 'all',
    hearing_status: 'scheduled',
    limit: 20,
  });
});

test('rows carry the case reference apart and stop at six', () => {
  const [first, second] = dashboardAgendaRows([hearing('h-1'), deadline], '-06:00');
  assert.equal(first.reference, 'RTA-2026-014');
  assert.deepEqual(first.meta, ['Defensa inicial']);
  assert.equal(first.date, '2026-01-02');
  assert.deepEqual(first.time, { datetime: '2026-01-02T11:00:00-06:00', text: '11:00' });
  assert.equal(first.badge, 'Audiencia');
  assert.equal(second.reference, 'NUC-123');
  assert.equal(second.title, 'Contestar demanda');
  assert.match(second.label, /^Plazo: Contestar demanda, NUC-123, 2 de enero de 2026 11:00/);
  const many = Array.from({ length: 9 }, (_, index) =>
    hearing(`h-${index}`, at.unix_seconds + index * 60),
  );
  assert.equal(dashboardAgendaRows(many, '-06:00').length, 6);
});

test('only the days with activities are named and marked', () => {
  const describe = dashboardAgendaDay([hearing('h-1'), hearing('h-2'), deadline], '-06:00');
  assert.deepEqual(describe('2026-01-02'), {
    label: '2 de enero de 2026: 3 actividades',
    marker: { dot: true },
    attrs: { 'data-agenda-day': '2026-01-02' },
  });
  assert.deepEqual(describe('2026-01-03'), { label: '3 de enero de 2026' });
});

function deferred() {
  let resolve, reject;
  const promise = new Promise((ok, fail) => {
    resolve = ok;
    reject = fail;
  });
  return { promise, resolve, reject };
}
function fakeScoped() {
  const pending = [];
  return {
    pending,
    disposed: false,
    list() {
      const call = deferred();
      pending.push(call);
      return call.promise;
    },
    dispose() {
      this.disposed = true;
    },
  };
}
const page = { items: [], complete: true };

test('a response reaches the overview only while its request, account and session are current', async () => {
  const scoped = fakeScoped();
  let user = { id: 'u-1', email: 'a@example.test', role: 'owner' },
    admit = true;
  const session = { principal: () => user, canAdmit: () => admit };
  const loader = dashboardAgendaLoader({ scoped, session });

  const answered = loader.load({});
  scoped.pending[0].resolve(page);
  assert.deepEqual(await answered, { page });

  const failed = loader.load({});
  scoped.pending[1].reject(new Error('Agenda no disponible'));
  assert.deepEqual(await failed, { error: 'Agenda no disponible' });

  const superseded = loader.load({});
  const latest = loader.load({});
  scoped.pending[2].resolve(page);
  scoped.pending[3].resolve(page);
  assert.equal(await superseded, null);
  assert.deepEqual(await latest, { page });

  const replaced = loader.load({});
  user = { id: 'u-2', email: 'b@example.test', role: 'litigator' };
  scoped.pending[4].resolve(page);
  assert.equal(await replaced, null);

  const closing = loader.load({});
  admit = false;
  scoped.pending[5].resolve(page);
  assert.equal(await closing, null);

  admit = true;
  const stopped = loader.load({});
  loader.stop();
  scoped.pending[6].reject(new Error('late'));
  assert.equal(await stopped, null);
  assert.equal(scoped.disposed, true);
  assert.equal(await loader.load({}), null);
});
