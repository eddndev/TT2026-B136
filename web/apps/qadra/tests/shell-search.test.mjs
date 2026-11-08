import test from 'node:test';
import assert from 'node:assert/strict';
import { foldText, matchCases } from '../src/lib/shell-search.mjs';

const rows = [
  {
    id: 'a',
    title: 'Olvera Casta\u00f1eda vs. Ram\u00edrez Lugo',
    reference: 'RTA-2026-014',
    penal_identifiers: { nuc: 'CI-FDS/T/0042/01-2026', judicial_case_number: '017/2026' },
  },
  { id: 'b', title: 'Amparo indirecto', reference: 'RTA-2026-021', penal_identifiers: null },
  { id: 'c', title: 'Juicio ordinario civil', reference: 'RTA-2026-027', penal_identifiers: null },
];

test('foldText ignores case, accents and surrounding space', () => {
  assert.equal(foldText('  Casta\u00f1eda R\u00c1MIREZ '), 'castaneda ramirez');
  assert.equal(foldText(undefined), '');
});

test('matchCases finds by title, reference and penal identifiers', () => {
  assert.deepEqual(
    matchCases(rows, 'castaneda').map((row) => row.id),
    ['a'],
  );
  assert.deepEqual(
    matchCases(rows, 'rta-2026-02').map((row) => row.id),
    ['b', 'c'],
  );
  assert.deepEqual(
    matchCases(rows, '017/2026').map((row) => row.id),
    ['a'],
  );
  assert.deepEqual(
    matchCases(rows, 'cI-fds').map((row) => row.id),
    ['a'],
  );
});

test('matchCases requires every word and ranks title prefixes first', () => {
  assert.deepEqual(
    matchCases(rows, 'amparo civil').map((row) => row.id),
    [],
  );
  assert.deepEqual(
    matchCases(rows, 'ju').map((row) => row.id),
    ['c'],
  );
  assert.deepEqual(
    matchCases(rows, 'i').map((row) => row.id),
    [],
    'a single character is not a query',
  );
  assert.equal(matchCases(rows, 'rta', 2).length, 2);
});
