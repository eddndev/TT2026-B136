import test from 'node:test';
import assert from 'node:assert/strict';
import { componentMarkup } from './deadline-editor-render.mjs';

test('deadline boolean starts without an invented applicability declaration', async () => {
  const html = await componentMarkup('DeadlineFieldsBoolean', {
    value: { kind: '' },
    label: 'Ambito',
  });
  assert.match(html, /Selecciona lo declarado/);
  assert.doesNotMatch(html, /value="(?:yes|no|unknown)" selected/);
});

test('unknown condition retains its explanation and false remains an explicit choice', async () => {
  const unknown = await componentMarkup('DeadlineFieldsBoolean', {
    value: { kind: 'unknown', reason: 'Falta la constancia' },
    label: 'Ambito',
  });
  assert.match(unknown, /Falta la constancia/);
  assert.match(unknown, /Motivo/);
  const rejected = await componentMarkup('DeadlineFieldsBoolean', {
    value: { kind: 'known', value: false },
    label: 'Ambito',
  });
  assert.match(rejected, /value="no" selected/);
  assert.doesNotMatch(rejected, /Motivo/);
});

test('client and paralegal cannot mount a deadline writer or fetch selectors', async () => {
  const api = new Proxy(
    {},
    {
      get() {
        throw new Error('Unauthorized API invocation');
      },
    },
  );
  for (const role of ['client', 'paralegal']) {
    const html = await componentMarkup('DeadlineEditor', {
      api,
      user: { id: '00000000-0000-0000-0000-000000000001', role },
      caseId: '00000000-0000-0000-0000-000000000002',
    });
    assert.doesNotMatch(html, /Formulario de plazo|Preparar plazo/);
  }
});

test('correction exposes missing profile conditions without inventing their declarations', async () => {
  const { definition, profile } = await import('./fixtures/deadline-unit.mjs');
  const html = await componentMarkup('DeadlineFields', {
    value: definition(),
    profile: profile(),
    caseId: '00000000-0000-0000-0000-000000000001',
    api: { deadlineProfiles: () => ({ dispose() {} }) },
  });
  assert.match(html, /Sin declaraci[o\u00f3]n guardada/);
  assert.match(html, /Agregar declaraci[o\u00f3]n de condici[o\u00f3]n/);
  assert.doesNotMatch(html, /Se cumple la condicion 1/);
});
