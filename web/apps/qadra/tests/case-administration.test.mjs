import test from 'node:test';
import assert from 'node:assert/strict';
import {
  caseDraft,
  caseValues,
  caseFilters,
  offenseValue,
} from '../src/lib/case-administration.mjs';
import { createApi } from '../src/lib/api.mjs';
const profile = {
  nuc: 'NUC-1',
  nuc_authority: 'Fiscalia',
  judicial_case_number: 'CJ-1',
  judicial_authority: 'Organo',
  offenses: ['Primero, segundo'],
  general_information: null,
  complementary_identifiers: null,
};
const values = { title: 'Caso', reference: 'REF', profile };
test('complete profile normalizes multiline text and preserves offense order and Unicode', () => {
  const draft = caseDraft({
    ...values,
    profile: {
      ...profile,
      offenses: [' Z ', '\u00c1'],
      general_information: ' \r\nUno\r\nDos\n ',
      complementary_identifiers: ' ',
    },
  });
  const result = caseValues(draft, true);
  assert.deepEqual(result.profile.offenses, ['Z', '\u00c1']);
  assert.equal(result.profile.general_information, 'Uno\nDos');
  assert.equal(result.profile.complementary_identifiers, null);
  assert.equal(result.title, 'Caso');
});
test('pending basic edits stay pending independently of revision', () => {
  for (const revision of [0, 1]) {
    assert.deepEqual(
      caseValues(caseDraft({ title: ' Caso ', reference: ' REF ', revision }), false),
      { title: 'Caso', reference: 'REF', profile: null },
    );
  }
  assert.throws(() => caseValues(caseDraft(), true));
});
test('field limits count scalar values and reject controls before trimming', () => {
  const draft = caseDraft(values);
  draft.title = '\u{1f642}'.repeat(200);
  assert.equal([...caseValues(draft, true).title].length, 200);
  draft.title += '\u{1f642}';
  assert.throws(() => caseValues(draft, true));
  for (const raw of ['\t', '\r', '\u0085']) {
    const invalid = caseDraft(values);
    invalid.profile.general_information = raw;
    assert.throws(() => caseValues(invalid, true));
    assert.throws(() => offenseValue(raw));
  }
  assert.equal(offenseValue(' \u00c1, B '), '\u00c1, B');
  assert.throws(() => offenseValue('x'.repeat(121)));
});
test('offenses are required, bounded and never silently deduplicated', () => {
  for (const offenses of [[], ['A', ' A '], Array(9).fill('A')]) {
    assert.throws(() =>
      caseValues(caseDraft({ ...values, profile: { ...profile, offenses } }), true),
    );
  }
  assert.equal(
    caseValues(
      caseDraft({
        ...values,
        profile: { ...profile, offenses: Array.from({ length: 8 }, (_, i) => String(i)) },
      }),
      true,
    ).profile.offenses.length,
    8,
  );
});
test('staff filters preserve literal text and validate independent states', () => {
  assert.deepEqual(
    caseFilters({
      status: 'closed',
      profile: 'pending',
      title: ' A% ',
      nuc: ' N ',
      judicial_case_number: '',
    }),
    { status: 'closed', profile: 'pending', title: 'A%', nuc: 'N' },
  );
  assert.throws(() => caseFilters({ status: 'archived', profile: 'all' }));
  assert.throws(() => caseFilters({ status: 'all', profile: 'ready' }));
});
test('administration facade uses exact bodies, cursors and disposed scopes', async () => {
  const calls = [];
  const detail = { id: 'case', administration: { case_id: 'case', revision: 0 } };
  const api = createApi(async (path, options) => {
    calls.push({ path, options });
    return new Response(
      JSON.stringify(
        path.includes('history')
          ? { revisions: [], has_more: false }
          : path.includes('case-administrations')
            ? { cases: [], has_more: false }
            : detail,
      ),
    );
  });
  await api.caseAdministrations({ afterId: 'last', status: 'all', profile: 'pending', nuc: 'A/B' });
  assert.equal(
    calls[0].path,
    '/api/v1/case-administrations?limit=50&after_id=last&status=all&profile=pending&nuc=A%2FB',
  );
  await api.createPenalCase(values);
  assert.deepEqual(JSON.parse(calls[1].options.body), values);
  const scoped = api.caseAdministration('case');
  await scoped.replace(0, { title: 'T', reference: 'R', profile: null });
  assert.deepEqual(JSON.parse(calls[2].options.body), {
    expected_revision: 0,
    title: 'T',
    reference: 'R',
    profile: null,
  });
  await scoped.status(1, 'closed');
  assert.deepEqual(JSON.parse(calls[3].options.body), {
    expected_revision: 1,
    administrative_status: 'closed',
  });
  await scoped.history({ beforeRevision: 4 });
  assert.match(calls[4].path, /before_revision=4/);
  scoped.dispose();
  await assert.rejects(scoped.get());
});
test('closed child mutation notifies only its current case without retry', async () => {
  let calls = 0,
    notices = 0;
  const api = createApi(async () => {
    calls++;
    return new Response(JSON.stringify({ error: { code: 'case_closed' } }), { status: 409 });
  });
  const unwatch = api.watchCase('case', () => notices++);
  await assert.rejects(api.caseDocuments('case').upload(new Blob(['x']), 'x'));
  assert.equal(notices, 1);
  assert.equal(calls, 1);
  unwatch();
  await assert.rejects(api.caseParticipants('case').create({}));
  assert.equal(notices, 1);
});
