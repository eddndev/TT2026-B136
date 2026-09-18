import {
  factObject as object,
  factInvalid as invalid,
  factUuid as uuid,
  factRevision as revision,
  factText as text,
  factLabel as label,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { calendarValues, calendarUrl } from './judicial-calendar-values.mjs';
import { civilDate, civilDays } from './judicial-calendar-time.mjs';
import { factTime } from './procedural-fact-time.mjs';
import {
  deadlineRequirement,
  deadlineRule,
  deadlineOutcome,
  deadlineRuleBlock,
} from './deadline-rule.mjs';
export function deadlineProfileScope(raw, caseId) {
  if (raw?.kind === 'case') {
    object(raw, ['kind', 'case_id']);
    uuid(raw.case_id);
    if (caseId !== undefined && raw.case_id !== caseId)
      invalid('El perfil pertenece a otro expediente.');
  } else {
    object(raw, ['kind', 'value']);
    if (raw.kind !== 'global') invalid();
    const v = raw.value;
    object(v, [
      'title',
      'jurisdiction',
      'entity_codes',
      'authority',
      'organ',
      'territory',
      'use_description',
    ]);
    for (const key of ['title', 'authority', 'organ', 'territory']) label(v[key]);
    text(v.use_description);
    if (
      !['federal', 'local'].includes(v.jurisdiction) ||
      !Array.isArray(v.entity_codes) ||
      v.entity_codes.length < 1 ||
      v.entity_codes.length > 32 ||
      v.entity_codes.some(
        (code, i) =>
          typeof code !== 'string' ||
          !/^(0[1-9]|[12]\d|3[0-2])$/.test(code) ||
          (i > 0 && code <= v.entity_codes[i - 1]),
      )
    )
      invalid();
  }
  return raw;
}
function unique(values, parse, minimum = 1) {
  if (!Array.isArray(values) || values.length < minimum || values.length > 16) invalid();
  values.forEach(parse);
  const ids = values.map((v) => (typeof v === 'string' ? uuid(v) : uuid(v.id)));
  if (new Set(ids).size !== ids.length) invalid();
  return new Set(ids);
}
function reference(raw) {
  object(raw, ['id', 'title', 'issuer', 'official_url', 'published_on', 'consulted_on', 'locator']);
  uuid(raw.id);
  label(raw.title);
  label(raw.issuer);
  calendarUrl(raw.official_url);
  civilDate(raw.consulted_on);
  text(raw.locator, 'Localizador', 512, false);
  if (raw.published_on !== null) {
    civilDate(raw.published_on);
    if (raw.published_on > raw.consulted_on) invalid();
  }
}
function referenceIds(raw, sources) {
  unique(raw, (id) => {
    uuid(id);
    if (!sources.has(id)) invalid();
  });
}
function completion(raw, sources, unit) {
  if (['arithmetic_instant', 'civil_candidate_only'].includes(raw?.kind)) object(raw, ['kind']);
  else {
    object(raw, ['kind', 'time', 'offset_seconds', 'from', 'through', 'channel', 'reference_id']);
    if (
      raw.kind !== 'civil_cutoff' ||
      !/^(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d$/.test(raw.time) ||
      !Number.isInteger(raw.offset_seconds) ||
      Math.abs(raw.offset_seconds) > 50400 ||
      raw.offset_seconds % 60
    )
      invalid();
    civilDate(raw.from);
    civilDate(raw.through);
    const days = civilDays(raw.from, raw.through);
    if (days < 1 || days > 1096) invalid();
    label(raw.channel);
    uuid(raw.reference_id);
    if (!sources.has(raw.reference_id)) invalid();
  }
  if ((unit === 'elapsed_hours') !== (raw.kind === 'arithmetic_instant')) invalid();
}
export function deadlineProfileDefinition(raw, caseId) {
  object(raw, [
    'title',
    'description',
    'scope',
    'references',
    'trigger',
    'template',
    'completion',
    'conditions',
    'examples',
  ]);
  label(raw.title);
  text(raw.description);
  deadlineProfileScope(raw.scope, caseId);
  deadlineRequirement(raw.trigger);
  const sources = unique(raw.references, reference),
    template = raw.template;
  let unit;
  if (template?.kind === 'fixed') {
    object(template, ['kind', 'rule']);
    deadlineRule(template.rule);
    unit = template.rule.kind;
  } else {
    object(template, ['kind', 'unit', 'maximum']);
    if (template.kind !== 'ordered') invalid();
    deadlineRule(template.unit, true);
    if (template.maximum !== null) revision(template.maximum);
    unit = template.unit.kind;
  }
  completion(raw.completion, sources, unit);
  unique(raw.conditions, (v) => {
    object(v, ['id', 'statement', 'reference_ids']);
    uuid(v.id);
    text(v.statement);
    referenceIds(v.reference_ids, sources);
  });
  unique(raw.examples, (v) => {
    object(v, [
      'id',
      'anchor',
      'ordered_quantity',
      'calendar',
      'expected',
      'reference_ids',
      'locator',
    ]);
    uuid(v.id);
    if (!same(factTime(v.anchor), v.anchor)) invalid();
    if (v.ordered_quantity !== null) revision(v.ordered_quantity);
    if (v.calendar !== null && !same(calendarValues(v.calendar), v.calendar)) invalid();
    if (v.expected?.kind === 'arithmetic') {
      object(v.expected, ['kind', 'outcome']);
      deadlineOutcome(v.expected.outcome);
    } else {
      object(v.expected, ['kind', 'block']);
      if (v.expected.kind !== 'rule_blocked') invalid();
      deadlineRuleBlock(v.expected.block);
    }
    referenceIds(v.reference_ids, sources);
    label(v.locator);
  });
  return raw;
}
