// The JSON description of one evidence validation that the panel offers for
// download: the engine, the policy and anchors used, the instants with their
// sources, the digests and every component with its findings. It describes
// the run; it is not a signature or a seal, and it never carries the bytes of
// the validated file. Rationale: the section "Interface" of
// docs/adr/0085-in-browser-evidence-validation.md.
import { HISTORICAL_NOTE } from './messages.mjs';

export const REPORT_FORMAT = 'qadra-evidence-validation/1';

const copy = (value) => (value === undefined ? null : JSON.parse(JSON.stringify(value)));

/** Build the exported report of one validation. */
export function exportReport({
  report,
  engine,
  policy,
  pinned,
  route,
  document,
  file,
  generatedAt,
}) {
  return {
    format: REPORT_FORMAT,
    generated_at: new Date(generatedAt).toISOString(),
    engine: {
      name: engine?.name ?? null,
      version: engine?.version ?? null,
      source_digest: engine?.source_digest ?? null,
    },
    document: { id: document.id, version: document.version },
    file: { name: file.name, size: file.size, sha256: file.sha256 },
    profile: report.profile ?? null,
    overall: report.overall,
    policy: {
      admitted_policies: copy(policy.admitted_policies),
      current_instant: policy.current_instant,
      clock: 'client',
    },
    anchors: { pinned: copy(pinned), route: route ? copy(route) : null },
    instants: copy(report.instants),
    digests: copy(report.digests),
    trust: copy(report.trust),
    components: copy(report.components),
    current_status: copy(report.current_status),
    note: HISTORICAL_NOTE,
  };
}

/** The download name of the exported report of a document version. */
export function reportFilename(document) {
  return `validacion-evidencia-${document.id}-v${document.version}.json`;
}
