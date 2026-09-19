// Synthetic arithmetic fixtures do not assert legal applicability.
import { randomUUID } from "node:crypto";
export const declaredDate = (day = 6) => ({
  precision: "date",
  year: 2026,
  month: 1,
  day,
  offset_seconds: null,
});
export const qualifiedTime = () => ({
  precision: "second",
  year: 2026,
  month: 1,
  day: 6,
  hour: 14,
  minute: 30,
  second: 7,
  offset_seconds: -21600,
});
export function calendarValues() {
  const source = randomUUID();
  return {
    scope: {
      title: "Calendario sintetico de plazos",
      jurisdiction: "local",
      authority: "Autoridad de prueba",
      organ: "Organo de prueba",
      territory: "Territorio declarado",
      entity_codes: ["09"],
      use_description:
        "Configuracion aritmetica sintetica sin aplicabilidad juridica afirmada",
    },
    coverage: { from: "2026-01-01", through: "2026-03-31" },
    sources: [
      {
        id: source,
        title: "Fuente sintetica de calendario",
        issuer: "Fixture",
        official_url: "https://example.org/deadline-calendar",
        published_on: null,
        consulted_on: "2026-01-01",
        locator: "Ejemplo aritmetico",
      },
    ],
    weekly_pattern: Array.from({ length: 7 }, (_, index) => ({
      weekday: index + 1,
      classification: "countable",
      source_ids: [source],
      explanation: "Dia declarado contable para la prueba",
    })),
    exceptions: [],
  };
}
export function profileDefinition(caseId, kind, calendar) {
  const reference = randomUUID();
  const titles = {
    daily: "Dias contables de prueba",
    monthly: "Mes civil de prueba",
    hourly: "Horas ordenadas de prueba",
  };
  const value = {
    title: titles[kind],
    description:
      "Regla sintetica reproducible sin afirmar aplicabilidad juridica",
    scope: { kind: "case", case_id: caseId },
    references: [
      {
        id: reference,
        title: "Referencia aritmetica sintetica",
        issuer: "Fixture",
        official_url: "https://example.org/deadline-profile",
        published_on: null,
        consulted_on: "2026-01-06",
        locator: "Ejemplo reproducible sin efecto juridico",
      },
    ],
    trigger: { kind: "source_field", field: "resolution_issued_at" },
    template: {
      kind: "fixed",
      rule: {
        kind: "days",
        quantity: 1,
        inclusion: "on_anchor",
        basis: "calendar_countable",
        final_day: "preserve",
      },
    },
    completion: {
      kind: "civil_cutoff",
      time: "17:30:00",
      offset_seconds: -21600,
      from: "2026-01-01",
      through: "2026-12-31",
      channel: "Canal sintetico declarado",
      reference_id: reference,
    },
    conditions: [
      {
        id: randomUUID(),
        statement: "Condicion declarada para probar el formulario",
        reference_ids: [reference],
      },
    ],
    examples: [
      {
        id: randomUUID(),
        anchor: declaredDate(),
        ordered_quantity: null,
        calendar: structuredClone(calendar),
        expected: {
          kind: "arithmetic",
          outcome: { kind: "civil_candidate", date: "2026-01-06" },
        },
        reference_ids: [reference],
        locator: "Ejemplo aritmetico",
      },
    ],
  };
  if (kind === "monthly") {
    value.template.rule = {
      kind: "civil_months",
      quantity: 1,
      final_day: "preserve",
    };
    value.examples[0].calendar = null;
    value.examples[0].expected.outcome.date = "2026-02-06";
  }
  if (kind === "hourly") {
    value.trigger = {
      kind: "qualified",
      family: "resolution",
      purpose: "ordered_period_start",
    };
    value.template = {
      kind: "ordered",
      unit: { kind: "elapsed_hours" },
      maximum: 72,
    };
    value.completion = { kind: "arithmetic_instant" };
    Object.assign(value.examples[0], {
      anchor: qualifiedTime(),
      ordered_quantity: 24,
      calendar: null,
      expected: {
        kind: "arithmetic",
        outcome: {
          kind: "instant_candidate",
          instant: {
            unix_seconds: 1767817807,
            nanosecond: 0,
            offset_seconds: 0,
          },
        },
      },
    });
  }
  return value;
}
export function registration(
  caseId,
  profile,
  source,
  calendar,
  responsible,
  title,
) {
  return {
    operation_id: randomUUID(),
    deadline_id: randomUUID(),
    change: {
      action: "register",
      expected_revision: 0,
      tracking: { profile: "follow", source: "fixed", calendar: "fixed" },
      definition: {
        title,
        profile: { id: profile.id, revision: profile.revision },
        responsible_id: responsible,
        input: {
          selection: {
            case_id: caseId,
            source: {
              kind: "known",
              value: {
                family: "resolution",
                id: source.id,
                revision: source.revision,
              },
            },
            qualification: null,
          },
          calendar: { id: calendar.id, revision: calendar.revision },
          ordered_quantity: null,
          qualification: {
            statement:
              "Aplicabilidad declarada para prueba sin efecto juridico",
            locator: "Fuente sintetica, pagina 1",
            scope_applies: { kind: "known", value: true },
            unresolved_incident: { kind: "known", value: false },
            conditions: profile.definition.conditions.map((row) => ({
              id: row.id,
              applies: { kind: "known", value: true },
              locator: "Declaracion sintetica",
            })),
          },
        },
      },
    },
  };
}
