import { createHash } from 'node:crypto';
import { caseId, document } from './helpers.mjs';
import { resourcePrepared, resourceRecord } from '../fixtures/procedural-resource-unit.mjs';
import { factAdministration, factResolutionSource } from '../fixtures/procedural-facts.mjs';

const digest = (value) => createHash('sha256').update(JSON.stringify(value)).digest('hex');
export function installResourceCommands(state) {
  state.resourcePrepare = (command) => {
    const history = state.resources.get(command.resource_id) || [],
      base = history.at(-1),
      change = command.change,
      prepared = resourcePrepared(command);
    const values = structuredClone(
      ['register', 'correct'].includes(change.action) ? change.values : base.values,
    );
    Object.assign(prepared, {
      case_id: caseId,
      values,
      recorded_by: { id: state.current.user.id, email: state.current.user.email },
      observed_administration: {
        ...structuredClone(factAdministration),
        revision: state.caseRevision,
        status: state.caseStatus,
      },
      observed_stage: { case_id: caseId, current: null },
      previous: base
        ? { revision: base.revision, capture_digest: base.receipt.capture_digest }
        : null,
    });
    const parent = [...state.factRecords.values()]
      .flat()
      .find(
        (row) => row.id === values.resolution.id && row.revision === values.resolution.revision,
      );
    if (!parent) throw new Error('Missing exact resolution fixture');
    const captures = (refs) =>
      [...new Map(refs.map((ref) => [`${ref.document_id}/${ref.version}`, ref])).values()]
        .sort((a, b) => a.document_id.localeCompare(b.document_id) || a.version - b.version)
        .map(({ locator, ...ref }) => ({
          ...ref,
          name: state.documents.get(ref.document_id)?.name || document.name,
          format: 'pdf',
          policy: 'pdf_docx_v1',
        }));
    prepared.sources = {
      resolution: factResolutionSource(parent),
      supports: captures([values.resolution_evidence]),
      appellants: values.appellants
        .map((value) => value.participant)
        .filter(Boolean)
        .sort((a, b) => a.id.localeCompare(b.id) || a.revision - b.revision)
        .map((ref) => {
          const row = state.records.get(ref.id)?.find((value) => value.revision === ref.revision);
          if (!row) throw new Error('Missing exact appellant fixture');
          return {
            case_id: caseId,
            ...ref,
            values_digest: row.values_digest,
            directory_status: row.directory_status,
            subject: row.subject
              ? {
                  id: row.subject.id,
                  revision: row.subject.revision,
                  values_digest: row.subject.values_digest,
                }
              : null,
            display_name: row.display_name,
            procedural_role: row.procedural_role,
            organization: row.organization || null,
            kind: row.profile?.kind || null,
          };
        }),
    };
    if (prepared.act) {
      prepared.act.revision = (change.expected_act_revision || 0) + 1;
      prepared.act.supports = captures(change.values.evidence);
      const previousAct = history.findLast((row) => row.act?.id === change.act_id);
      prepared.act.previous =
        change.action === 'correct_act'
          ? { revision: previousAct.revision, capture_digest: previousAct.receipt.capture_digest }
          : null;
    }
    prepared.submission_digest = digest({ command, values, previous: prepared.previous });
    return prepared;
  };
  state.resourceCommit = (prepared) => {
    const row = resourceRecord(prepared);
    row.receipt.capture_digest = digest(row);
    state.resources.set(row.id, [...(state.resources.get(row.id) || []), row]);
    return row;
  };
}
