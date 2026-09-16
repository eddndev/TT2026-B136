import { caseText } from './case-administration.mjs';
import { civilDate, civilDays } from './judicial-calendar-time.mjs';
export const maximumCalendarRevision = 4294967295;
const invalid = (message) => {
  throw new Error(message);
};
export function calendarUuid(value) {
  if (typeof value !== 'string' || !/^[a-f\d]{8}(-[a-f\d]{4}){3}-[a-f\d]{12}$/i.test(value))
    invalid('Consulta una identidad exacta v\u00e1lida.');
  return value.toLowerCase();
}
export function calendarText(value, label, limit, multiline = false) {
  if (typeof value !== 'string' || /[\ud800-\udfff]/u.test(value))
    invalid(`${label}: revisa el texto.`);
  return caseText(value, { key: label, label, limit, multiline, required: true });
}
export function calendarUrl(raw) {
  if (typeof raw !== 'string' || /[^\x20-\x7e]/.test(raw))
    invalid('La referencia requiere una URL HTTPS ASCII sin controles.');
  const value = raw.trim();
  const match = /^https:\/\/([^/?#]+)([/?#].*)?$/.exec(value);
  if (!match || value.length > 2048) invalid('Revisa la referencia HTTPS declarada.');
  const host = match[1],
    labels = host.split('.'),
    rest = match[2] || '';
  if (
    host.length > 253 ||
    labels.length < 2 ||
    labels.some(
      (l) => l.length > 63 || !/^([a-z\d]|[a-z\d][a-z\d-]*[a-z\d])$/i.test(l) || /^xn--/i.test(l),
    ) ||
    !/^[a-z]{2,63}$/i.test(labels.at(-1))
  )
    invalid('Usa un host DNS completo, sin puerto, IP ni usuario.');
  if (
    (rest.match(/#/g) || []).length > 1 ||
    !/^(?:[a-z\d\-._~!$&'()*+,;=:@/?#]|%[a-f\d]{2})*$/i.test(rest)
  )
    invalid('Revisa la ruta y los escapes de la referencia HTTPS.');
  return value;
}
export function sameCalendarData(a, b) {
  if (a === b) return true;
  if (
    a == null ||
    b == null ||
    typeof a !== 'object' ||
    typeof b !== 'object' ||
    Array.isArray(a) !== Array.isArray(b)
  )
    return false;
  const ak = Object.keys(a).sort(),
    bk = Object.keys(b).sort();
  return (
    ak.length === bk.length &&
    ak.every((key, i) => key === bk[i] && sameCalendarData(a[key], b[key]))
  );
}
function unique(raw, limit, label, convert, key = (v) => v) {
  if (!Array.isArray(raw) || raw.length > limit)
    invalid(`Revisa ${label}; el l\u00edmite es ${limit}.`);
  const values = raw.map(convert);
  if (new Set(values.map(key)).size !== values.length)
    invalid(`Hay referencias duplicadas en ${label}.`);
  return values;
}
function scope(raw) {
  if (!raw || !['federal', 'local'].includes(raw.jurisdiction))
    invalid('Selecciona el fuero del calendario.');
  const entity_codes = unique(raw.entity_codes, 32, 'entidades', (v) => {
    if (typeof v !== 'string' || !/^(0[1-9]|[12]\d|3[0-2])$/.test(v))
      invalid('Selecciona una clave de entidad de 01 a 32.');
    return v;
  }).sort();
  if (!entity_codes.length)
    invalid('Selecciona al menos una entidad; no hay alcance nacional predeterminado.');
  return {
    title: calendarText(raw.title, 'T\u00edtulo', 200),
    jurisdiction: raw.jurisdiction,
    entity_codes,
    authority: calendarText(raw.authority, 'Autoridad', 200),
    organ: calendarText(raw.organ, '\u00d3rgano', 200),
    territory: calendarText(raw.territory, 'Territorio', 200),
    use_description: calendarText(raw.use_description, 'Uso declarado', 1000, true),
  };
}
function rule(raw, sources) {
  if (!['countable', 'excluded', 'unresolved'].includes(raw?.classification))
    invalid('Selecciona la clasificaci\u00f3n de cada regla.');
  const source_ids = unique(raw.source_ids, 16, 'fuentes de la regla', calendarUuid).sort();
  if (
    (raw.classification !== 'unresolved' && !source_ids.length) ||
    source_ids.some((id) => !sources.has(id))
  )
    invalid('Cada regla computable o excluida requiere fuentes de esta revisi\u00f3n.');
  return {
    classification: raw.classification,
    source_ids,
    explanation: calendarText(raw.explanation, 'Explicaci\u00f3n de la regla', 256, true),
  };
}
export function calendarValues(raw) {
  const declaredScope = scope(raw.scope),
    from = civilDate(raw.coverage?.from),
    through = civilDate(raw.coverage?.through);
  if (civilDays(from, through) < 1 || civilDays(from, through) > 1096)
    invalid('La cobertura debe abarcar de 1 a 1096 d\u00edas inclusivos.');
  const sources = unique(
    raw.sources,
    16,
    'fuentes',
    (s) => {
      const published_on = s.published_on ? civilDate(s.published_on) : null,
        consulted_on = civilDate(s.consulted_on);
      if (published_on && published_on > consulted_on)
        invalid('La publicaci\u00f3n no puede ser posterior a la consulta declarada.');
      return {
        id: calendarUuid(s.id),
        title: calendarText(s.title, 'T\u00edtulo de fuente', 200),
        issuer: calendarText(s.issuer, 'Emisor', 200),
        official_url: calendarUrl(s.official_url),
        published_on,
        consulted_on,
        locator: calendarText(s.locator, 'Localizador', 512),
      };
    },
    (s) => s.id,
  ).sort((a, b) => a.id.localeCompare(b.id));
  const sourceIds = new Set(sources.map((s) => s.id));
  const weekly_pattern = unique(
    raw.weekly_pattern,
    7,
    'patr\u00f3n semanal',
    (r) => {
      if (!Number.isInteger(r.weekday) || r.weekday < 1 || r.weekday > 7)
        invalid('Usa cada d\u00eda de la semana una sola vez.');
      return { weekday: r.weekday, ...rule(r, sourceIds) };
    },
    (r) => r.weekday,
  ).sort((a, b) => a.weekday - b.weekday);
  if (weekly_pattern.length !== 7) invalid('Declara las siete reglas del patr\u00f3n semanal.');
  const exceptions = unique(
    raw.exceptions,
    64,
    'excepciones',
    (r) => ({
      id: calendarUuid(r.id),
      from: civilDate(r.from),
      through: civilDate(r.through),
      ...rule(r, sourceIds),
    }),
    (r) => r.id,
  ).sort(
    (a, b) =>
      a.from.localeCompare(b.from) ||
      a.through.localeCompare(b.through) ||
      a.id.localeCompare(b.id),
  );
  exceptions.forEach((r, i) => {
    if (
      r.from < from ||
      r.through > through ||
      r.through < r.from ||
      (i && r.from <= exceptions[i - 1].through)
    )
      invalid('Las excepciones deben estar dentro de la cobertura y no solaparse.');
  });
  return { scope: declaredScope, coverage: { from, through }, sources, weekly_pattern, exceptions };
}
export function calendarDraft(record) {
  return {
    ...structuredClone(
      record?.values || {
        scope: {
          title: '',
          jurisdiction: '',
          entity_codes: [],
          authority: '',
          organ: '',
          territory: '',
          use_description: '',
        },
        coverage: { from: '', through: '' },
        sources: [],
        weekly_pattern: Array.from({ length: 7 }, (_, i) => ({
          weekday: i + 1,
          classification: '',
          source_ids: [],
          explanation: '',
        })),
        exceptions: [],
      },
    ),
    reason: '',
  };
}
export function calendarCommand(draft, { action, base, calendarId, operationId }) {
  const calendar_id = calendarUuid(calendarId),
    operation_id = calendarUuid(operationId);
  let change;
  if (action === 'publish') {
    if (base) invalid('Publicar requiere un calendario nuevo.');
    change = { action, expected_revision: 0, values: calendarValues(draft) };
  } else {
    if (
      !['replace', 'retire'].includes(action) ||
      !base ||
      base.id !== calendar_id ||
      base.status !== 'published'
    )
      invalid('Consulta una cabeza publicada antes de cambiarla.');
    const expected_revision = base.revision;
    if (
      !Number.isInteger(expected_revision) ||
      expected_revision < 1 ||
      expected_revision >= maximumCalendarRevision
    )
      invalid('No hay otra revisi\u00f3n disponible para este calendario.');
    change = { action, expected_revision };
    if (action === 'replace') {
      const values = calendarValues(draft);
      if (!sameCalendarData(values.scope, base.values.scope))
        invalid('El \u00e1mbito queda fijo desde la publicaci\u00f3n inicial.');
      change.values = values;
    }
    change.reason = calendarText(draft.reason, 'Motivo', 1000, true);
  }
  return { operation_id, calendar_id, change };
}
