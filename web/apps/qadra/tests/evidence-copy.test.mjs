// The texts the evidence validator shows are written in Spanish with its
// accents. The sources stay ASCII by writing accented letters as \u escapes
// in scripts and as numeric character references in Svelte markup, as the
// rest of the interface does.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import {
  FAMILY_FALLBACKS,
  HISTORICAL_NOTE,
  NOT_QADRA_EXPORT,
  findingMessage,
  findingMessages,
} from '../src/lib/evidence/messages.mjs';
import {
  PURPOSE_LABELS,
  STAGE_LABELS,
  componentView,
  instantView,
  overallLabel,
  profileLabel,
} from '../src/lib/evidence/presentation.mjs';

const app = fileURLToPath(new URL('../', import.meta.url));

// Spanish words that need an accent; none of them is also an English word
// of the interface code.
const UNACCENTED = [
  'raiz',
  'raices',
  'validacion',
  'revision',
  'politica',
  'revocacion',
  'danado',
  'tamano',
  'exporto',
  'ningun',
  'vinculacion',
  'declaracion',
  'aceptacion',
  'historicos',
  'tecnicos',
  'mas',
  'encontro',
  'demas',
  'abrio',
  'reconocio',
  'mostro',
  'emitio',
  'revoco',
  'calculo',
  'reviso',
  'firmo',
  'quedo',
  'envia',
  'limite',
  'numero',
  'asi',
  'valida',
  'valido',
  'vacio',
  'metodo',
  'todavia',
  'despues',
  'comprobacion',
];
const PATTERN = new RegExp(`\\b(?:${UNACCENTED.join('|')})\\b`, 'i');
// "esta" before these words is the verb "est\u00e1".
const VERB_ESTA = /\besta (?:en|danad|revocad|vigente|fija|vac)/i;

function check(text, where) {
  assert.ok(!PATTERN.test(text), `${where}: "${text}" lacks an accent (${PATTERN.exec(text)})`);
  assert.ok(!VERB_ESTA.test(text), `${where}: "${text}" lacks the accent of est\u00e1`);
  assert.ok(!/[\u2013\u2014]/.test(text), `${where}: "${text}" has a long dash`);
}

test('the messages of the findings are written with accents', () => {
  for (const [code, text] of Object.entries(findingMessages)) check(text, code);
  for (const [family, text] of Object.entries(FAMILY_FALLBACKS)) check(text, family);
  check(HISTORICAL_NOTE, 'HISTORICAL_NOTE');
  check(NOT_QADRA_EXPORT, 'NOT_QADRA_EXPORT');
  check(findingMessage('unknown'), 'unknown');
});

test('the labels of the result are written with accents', () => {
  for (const text of [...Object.values(STAGE_LABELS), ...Object.values(PURPOSE_LABELS)])
    check(text, 'label');
  for (const overall of ['checked', 'indeterminate', 'failed', 'not-supported']) {
    const { label, summary } = overallLabel({ overall });
    check(label, overall);
    check(summary, overall);
  }
  for (const profile of ['server-seal-v1', 'individual-signing-v1', 'individual-signing-v2', 'x'])
    check(profileLabel(profile), profile);
  const kinds = ['package', 'document-token', 'document-signature', 'binding-consent'];
  kinds.push('certificates', 'revocation', 'manifest', 'context', 'other');
  for (const kind of kinds)
    for (const outcome of ['checked', 'failed', 'indeterminate', 'not-supported']) {
      const view = componentView({ kind, outcome, findings: [] });
      check(view.title, kind);
      check(view.status, kind);
    }
  for (const label of ['document token genTime', 'binding_registered_at', 'accepted_at', 'x'])
    for (const source of ['token', 'application-record', 'caller', 'x']) {
      const view = instantView({ label, unix: 0, source });
      check(view.label, label);
      check(view.source, source);
    }
});

const sourcePath = (name) => `${app}${name}`;

/** The visible text of a file: its string literals and, in Svelte, its markup. */
function visible(name) {
  const source = readFileSync(sourcePath(name), 'utf8');
  const decoded = source
    .replace(/\\u([0-9a-f]{4})/gi, (_, hex) => String.fromCharCode(parseInt(hex, 16)))
    .replace(/&#(\d+);/g, (_, code) => String.fromCharCode(Number(code)));
  const svelte = name.endsWith('.svelte');
  const script = svelte ? /<script>([\s\S]*?)<\/script>/.exec(decoded)?.[1] || '' : decoded;
  const strings = [...script.matchAll(/'([^'\n]*\s[^'\n]*)'/g)].map((match) => match[1]);
  const markup = decoded
    .replace(/<script>[\s\S]*?<\/script>/, '')
    .replace(/<style>[\s\S]*?<\/style>/, '')
    .replace(/\{[^{}]*\}/g, ' ')
    .replace(/<[^>]*>/g, ' ');
  return svelte ? [...strings, markup] : strings;
}

test('the panel and the result are written with accents', () => {
  for (const name of [
    'src/components/EvidenceValidation.svelte',
    'src/components/EvidenceValidationResult.svelte',
    'src/lib/evidence-verify-client.mjs',
  ])
    for (const text of visible(name)) check(text, name);
});
