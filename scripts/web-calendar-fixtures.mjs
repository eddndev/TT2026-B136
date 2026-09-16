// Provision global calendar scenarios in disposable browser services.
import { randomUUID } from "node:crypto";
export async function provisionCalendars(request) {
  const fixture = {};
  for (const role of ["owner", "litigator", "paralegal", "client"]) {
    const password = `calendar browser ${role} password`;
    const enrollment = await request(
      "POST",
      "/users",
      {
        email: `calendars.${role}@example.com`,
        password,
        role,
      },
      201,
    );
    fixture[role] = {
      id: enrollment.user.id,
      email: enrollment.user.email,
      password,
      recoveryCodes: enrollment.recovery_codes,
    };
  }
  for (const [key, title] of [
    ["read", "Calendario de consulta global"],
    ["race", "Calendario de concurrencia"],
  ]) {
    const sourceId = randomUUID();
    const command = {
      operation_id: randomUUID(),
      calendar_id: randomUUID(),
      change: {
        action: "publish",
        expected_revision: 0,
        values: {
          scope: {
            title,
            jurisdiction: "local",
            authority: "Autoridad declarada de prueba",
            organ: "Organo declarado de prueba",
            territory: "Territorio declarado",
            use_description:
              "Configuracion sintetica sin aplicabilidad juridica afirmada",
            entity_codes: ["01"],
          },
          coverage: { from: "2000-02-27", through: "2000-03-04" },
          sources: [
            {
              id: sourceId,
              title: "Fuente sintetica",
              issuer: "Emisor de prueba",
              official_url: "https://example.org/calendar",
              published_on: null,
              consulted_on: "2000-02-01",
              locator: "Referencia de prueba sin copia archivada",
            },
          ],
          weekly_pattern: Array.from({ length: 7 }, (_, i) => ({
            weekday: i + 1,
            classification: i === 0 ? "countable" : "unresolved",
            source_ids: i === 0 ? [sourceId] : [],
            explanation: "Regla declarada para prueba",
          })),
          exceptions: [
            {
              id: randomUUID(),
              from: "2000-02-29",
              through: "2000-02-29",
              classification: "excluded",
              source_ids: [sourceId],
              explanation: "Excepcion de prueba",
            },
          ],
        },
      },
    };
    const prepared = await request(
      "POST",
      "/judicial-calendars/prepare",
      command,
    );
    fixture[key] = await request(
      "POST",
      "/judicial-calendars",
      {
        command: prepared.command,
        expected_submission_digest: prepared.submission_digest,
      },
      201,
    );
  }
  return fixture;
}
