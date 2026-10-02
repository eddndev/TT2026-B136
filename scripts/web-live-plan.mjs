// Keep fixture ownership explicit; see docs/adr/0054-browser-fixture-partitions.md.
import { readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

export const fixtureNames = [
  "participants",
  "caseAdministration",
  "caseStages",
  "hearings",
  "alerts",
  "hearingResults",
  "proceduralFacts",
  "proceduralResources",
  "resourceActivities",
  "documentContent",
  "members",
  "dashboard",
  "caseReports",
  "judicialCalendars",
  "deadlines",
  "deadlineReevaluation",
  "combinedAgenda",
];

const families = [
  [/^(case-stages|stage-adoption)\./, 1, ["caseStages"]],
  [/^case-administration\./, 3, ["caseAdministration"]],
  [/^dashboard\./, 3, ["dashboard"]],
  [/^case-reports\./, 3, ["caseReports"]],
  [/^deadline-reevaluation\./, 1, ["deadlineReevaluation"]],
  [/^deadline-/, 1, ["deadlines"]],
  [/^combined-agenda\./, 1, ["deadlineReevaluation", "combinedAgenda"]],
  [/^document-workflow\./, 1, []],
  [/^document-classification\./, 3, []],
  [/^alerts\./, 2, ["hearings", "alerts"]],
  [/^hearing-result-/, 2, ["hearingResults"]],
  [/^hearings?[-.]/, 2, ["hearings"]],
  [/^document-content\./, 2, ["documentContent"]],
  [/^document-admission-real\./, 2, ["documentContent"]],
  [/^members\./, 2, ["members"]],
  [/^judicial-calendars\./, 2, ["judicialCalendars"]],
  [/^(case-participants|typed-participant)/, 3, ["participants"]],
  [/^procedural-facts-/, 3, ["proceduralFacts"]],
  [/^procedural-resources\./, 3, ["proceduralResources"]],
  [/^resource-activities-real\./, 3, ["resourceActivities"]],
  [/^activity-resources-real\./, 3, ["resourceActivities"]],
  [/^resource-activities-contextual-real\./, 3, ["resourceActivities"]],
];

export function planLiveSuite(files, partition) {
  const sorted = [...files].sort();
  if (!partition) return { files: sorted, fixtures: [...fixtureNames] };
  if (!/^[123]\/3$/.test(partition))
    throw new Error("Invalid live browser partition");
  const selected = [],
    required = new Set();
  for (const file of sorted) {
    const family = families.find(([pattern]) =>
      pattern.test(file.split("/").at(-1)),
    );
    const fallback = [...file].reduce(
      (value, char) => (value * 31 + char.charCodeAt(0)) >>> 0,
      0,
    );
    const owner = family?.[1] ?? (fallback % 3) + 1;
    if (owner !== Number(partition[0])) continue;
    selected.push(file);
    for (const name of family?.[2] ?? fixtureNames) required.add(name);
  }
  return {
    files: selected,
    fixtures: fixtureNames.filter((name) => required.has(name)),
  };
}

export function discoverLiveSpecs(
  directory = new URL("../web/tests/live/", import.meta.url),
) {
  return readdirSync(directory, { recursive: true })
    .map((file) => file.replaceAll("\\", "/"))
    .filter((file) => /\.(?:spec|test)\.[cm]?[jt]sx?$/.test(file))
    .sort();
}

export function currentLivePlan() {
  return planLiveSuite(discoverLiveSpecs(), process.env.TT_WEB_LIVE_SHARD);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const plan = currentLivePlan();
  if (process.argv[2] === "--has")
    process.exitCode = plan.fixtures.includes(process.argv[3]) ? 0 : 1;
  else console.log(JSON.stringify(plan));
}
