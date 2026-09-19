// Independent authorized cases combine a hearing with a followed deadline.
import { randomUUID } from "node:crypto";
import { provisionFiveCaseAgenda } from "./web-agenda-five-case-fixtures.mjs";
import {
  persistFollowRecord,
  provisionDeadlineReevaluation,
} from "./web-deadline-reevaluation.mjs";

export async function provisionCombinedAgenda(call) {
  const scenarios = await provisionDeadlineReevaluation(call, "agenda-");
  for (const scenario of Object.values(scenarios)) {
    const route = `/cases/${scenario.case.id}`;
    const current = await call("GET", `${route}/deadlines/${scenario.deadline.id}`);
    const due = current.operational.due_at;
    if (!due || due.nanosecond !== 0 || due.offset_seconds !== 0)
      throw new Error("Combined agenda fixture requires a current whole-second UTC due");
    const scheduledAt = new Date(due.unix_seconds * 1000).toISOString().replace(".000Z", "Z");
    const context = await call("GET", `${route}/hearings/context`);
    scenario.hearing = await persistFollowRecord(call, `${route}/hearings`, {
      operation_id: randomUUID(),
      hearing_id: randomUUID(),
      change: {
        action: "schedule",
        expected_revision: 0,
        expected_case_revision: context.case_revision,
        expected_stage_revision: context.stage_revision,
        values: {
          kind: "initial",
          scheduled_at: scheduledAt,
          modality: "in_person",
          venue: "Sede sintetica de agenda combinada",
          note: "Nota privada excluida del resumen de agenda",
          participants: [],
          conviction_basis: null,
        },
      },
    });
    scenario.date = scheduledAt.slice(0, 10);
    scenario.dueAt = due;
  }
  scenarios.fiveCases = await provisionFiveCaseAgenda(
    call,
    scenarios.desktop.operator,
    scenarios.desktop.calendar,
  );
  return scenarios;
}
