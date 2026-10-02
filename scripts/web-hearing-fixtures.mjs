import { provisionCaseReports } from "./web-case-report-fixtures.mjs";
import { provisionDashboard } from "./web-dashboard-fixtures.mjs";
import { provisionMembers } from "./web-member-fixtures.mjs";
import { provisionDocumentContent } from "./web-document-content-fixtures.mjs";
import { provisionDeadlines } from "./web-deadline-fixtures.mjs";
import { provisionDeadlineReevaluation } from "./web-deadline-reevaluation.mjs";
import { provisionCombinedAgenda } from "./web-combined-agenda-fixtures.mjs";
import { provisionAlerts } from "./web-alert-fixtures.mjs";
import { provisionProceduralFacts } from "./web-procedural-fact-fixtures.mjs";
import { provisionResources } from "./web-resource-fixtures.mjs";
import { provisionResourceActivities } from "./web-resource-activity-fixtures.mjs";
// Provision hearing scenarios using independent accounts in disposable services.
import { provisionCalendars } from "./web-calendar-fixtures.mjs";
import { provisionHearingResults } from "./web-hearing-result-fixtures.mjs";
import { provisionFixtures } from "./web-fixture-provisioning.mjs";
import { provisionHearings } from "./web-hearing-base-fixture.mjs";
import { currentLivePlan } from "./web-live-plan.mjs";
import { readFile, writeFile } from "node:fs/promises";
const needed = new Set(currentLivePlan().fixtures);
async function measured(name, run) {
  return (await provisionFixtures({ [name]: run }, 1))[name];
}
const fixturePath = process.env.TT_WEB_FIXTURES,
  base = process.env.API_PROXY_TARGET;
if (!fixturePath || !base || !process.env.IDENTITY_TEST_DATABASE_URL)
  throw new Error("disposable browser services are required");
const fixture = JSON.parse(await readFile(fixturePath, "utf8"));
let token;
async function request(method, path, body, expected = 200, headers = {}) {
  const binary = Buffer.isBuffer(body);
  const response = await fetch(`${base}/api/v1${path}`, {
    method,
    headers: {
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(body !== undefined
        ? {
            "Content-Type": binary
              ? "application/octet-stream"
              : "application/json",
          }
        : {}),
      ...headers,
    },
    body: body === undefined ? undefined : binary ? body : JSON.stringify(body),
  });
  if (response.status !== expected)
    throw new Error(
      `hearing fixture ${method} ${path}: expected ${expected}, got ${response.status}`,
    );
  return response.status === 204 ? undefined : response.json();
}
// Stage setup consumes bootstrap code 6; its Owner reserves code 7 for provisioning.
// Without stage setup, bootstrap code 6 remains available for these families.
const provisioner = fixture.caseStages?.owner ?? fixture;
const recoveryCode = provisioner.recoveryCodes[fixture.caseStages ? 7 : 6];
if (typeof recoveryCode !== "string" || recoveryCode.length === 0)
  throw new Error("Fixture provisioner has no reserved recovery code");
const challenge = await request("POST", "/auth/login", {
  email: provisioner.email,
  password: provisioner.password,
});
const session = await request("POST", "/auth/mfa/recovery", {
  challenge_token: challenge.challenge_token,
  code: recoveryCode,
});
token = session.access_token;
const pdf = await readFile(
  new URL(
    "../crates/infrastructure/tests/fixtures/stage-support.pdf",
    import.meta.url,
  ),
);
try {
  if (needed.has("hearings")) {
    fixture.hearings = await measured("hearings", () =>
      provisionHearings(request, pdf),
    );
  }
  if (needed.has("alerts"))
    fixture.alerts = await measured("alerts", () =>
      provisionAlerts(request, fixture.hearings),
    );
  Object.assign(
    fixture,
    await provisionFixtures(
      Object.fromEntries(
        Object.entries({
          hearingResults: () => provisionHearingResults(request),
          proceduralFacts: () => provisionProceduralFacts(request),
          proceduralResources: () => provisionResources(request),
          resourceActivities: () => provisionResourceActivities(request),
          documentContent: () => provisionDocumentContent(request),
          members: () => provisionMembers(request),
          dashboard: () => provisionDashboard(request),
          caseReports: () => provisionCaseReports(request),
          judicialCalendars: () => provisionCalendars(request),
          deadlines: () => provisionDeadlines(request),
        }).filter(([name]) => needed.has(name)),
      ),
      Number(process.env.TT_WEB_FIXTURE_WORKERS || "1"),
    ),
  );
  if (process.env.TT_DEADLINE_REEVALUATION_ACCEPTANCE === "1") {
    if (needed.has("deadlineReevaluation"))
      fixture.deadlineReevaluation = await measured(
        "deadlineReevaluation",
        () => provisionDeadlineReevaluation(request),
      );
    if (needed.has("combinedAgenda"))
      fixture.combinedAgenda = await measured("combinedAgenda", () =>
        provisionCombinedAgenda(request),
      );
  }
  await writeFile(fixturePath, `${JSON.stringify(fixture)}\n`, { mode: 0o600 });
} finally {
  await request("POST", "/auth/logout", undefined, 204);
}
