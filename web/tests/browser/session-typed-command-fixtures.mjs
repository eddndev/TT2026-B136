import { createHash } from 'node:crypto';
import { caseId } from './helpers.mjs';
import { subject, typed } from './typed-participant-helpers.mjs';

export const createdParticipantId = '77777777-7777-4777-8777-777777777777';
export const createdSubjectId = '99999999-9999-4999-8999-999999999999';
const digest = (value) => createHash('sha256').update(JSON.stringify(value)).digest('hex');

export async function typedCommand(route, call, state, { reply, wait, unexpected }) {
  const body = route.request().postDataJSON();
  if (call.path.endsWith('/review')) {
    if (!state.typedReviewBudget) return unexpected(route, call);
    state.typedReviewBudget--;
    state.typedReviews.push({ ...call, values: body });
    const original =
      body.participant.operation === 'existing'
        ? state.records.get(body.participant.id)?.at(-1)
        : null;
    if (original && body.participant.expected_revision !== original.revision)
      return reply(route, { error: { code: 'participant_revision_conflict' } }, 409);
    const reference = body.subject.reference;
    const identity =
      body.subject.operation === 'keep'
        ? state.subjects.get(reference.id)?.find((row) => row.revision === reference.revision)
        : { ...subject, id: createdSubjectId, values: body.subject.values };
    if (!identity) return reply(route, { error: { code: 'subject_not_found' } }, 404);
    const proposal = {
      subject:
        body.subject.operation === 'keep'
          ? body.subject
          : {
              operation: 'append',
              id: createdSubjectId,
              expected_revision: 0,
              values: identity.values,
            },
      participant_id: original?.id ?? createdParticipantId,
      expected_participant_revision: original?.revision ?? 0,
      values: {
        subject: {
          id: identity.id,
          revision: identity.revision,
          values_digest: identity.values_digest,
        },
        directory_status: original?.directory_status ?? 'active',
        role: body.role,
      },
    };
    await wait(call);
    return reply(route, {
      case_id: caseId,
      proposal,
      directory_stamp: state.directoryStamp,
      candidates: state.typedCandidates,
    });
  }
  if (call.path.endsWith('/prepare')) {
    if (!state.typedPrepareBudget) return unexpected(route, call);
    state.typedPrepareBudget--;
    const hash = digest(body);
    const bytes = Buffer.alloc(218);
    bytes.write('PCRED1');
    Buffer.from(hash, 'hex').copy(bytes, 8);
    const declaration = body.certificate_base64
      ? {
          bytes_base64: bytes.toString('base64'),
          digest: hash,
          participant_values_digest: digest(body.proposal.values),
          certificate: {
            der_base64: body.certificate_base64,
            fingerprint: 'a'.repeat(64),
            summary: {
              subject: 'Test',
              issuer: 'CA interna',
              serial_hex: '01',
              not_before_unix: 1,
              not_after_unix: 2000000000,
            },
          },
          deployment_id: caseId,
          trust_revision: 1,
          root_fingerprint: 'b'.repeat(64),
          policy: 'internal_demo_v1',
        }
      : null;
    const result = {
      case_id: caseId,
      ...body,
      declaration,
      submission_revision: body.proposal.expected_participant_revision + 1,
      submission_digest: declaration ? null : hash,
    };
    state.typedPreparations.push({ ...call, values: body, result });
    await wait(call);
    return reply(route, result);
  }
  if (!call.path.endsWith('/commit') || !state.nextTypedCommit) return unexpected(route, call);
  const mode = state.nextTypedCommit;
  state.nextTypedCommit = null;
  state.typedCommits.push({ ...call, values: body });
  const preparation = state.typedPreparations.findLast(
    (entry) => JSON.stringify(entry.values) === JSON.stringify(body.prepared),
  );
  if (!preparation) return reply(route, { error: { code: 'unprepared_test_commit' } }, 400);
  if (state.caseStatus === 'closed') return reply(route, { error: { code: 'case_closed' } }, 409);
  const prepared = preparation.result,
    proposal = prepared.proposal;
  const rows = state.records.get(proposal.participant_id) ?? [];
  if ((rows.at(-1)?.revision ?? 0) !== proposal.expected_participant_revision)
    return reply(route, { error: { code: 'participant_revision_conflict' } }, 409);
  const identity =
    proposal.subject.operation === 'keep'
      ? state.subjects
          .get(proposal.subject.reference.id)
          .find((row) => row.revision === proposal.subject.reference.revision)
      : { ...subject, id: proposal.subject.id, values: proposal.subject.values };
  const result = {
    ...typed,
    ...proposal.values.role,
    id: proposal.participant_id,
    revision: prepared.submission_revision,
    subject: identity,
    display_name: identity.values.name.value || identity.values.name,
    procedural_role: proposal.values.role.profile.kind,
    directory_status: proposal.values.directory_status,
    submission_revision: prepared.submission_revision,
    submission_digest: prepared.submission_digest,
    credential_origin: prepared.declaration
      ? {
          participant_id: proposal.participant_id,
          participant_revision: prepared.submission_revision,
          statement_digest: prepared.declaration.digest,
        }
      : null,
  };
  if (mode.commit !== false) {
    state.records.set(result.id, [...rows, result]);
    if (proposal.subject.operation === 'append') state.subjects.set(identity.id, [identity]);
    if (prepared.declaration)
      state.credentialEvidence.set(`${result.id}:${result.revision}`, {
        case_id: caseId,
        reference: result.credential_origin,
        statement_digest: prepared.declaration.digest,
        declaration_base64: prepared.declaration.bytes_base64,
        certificate_fingerprint: prepared.declaration.certificate.fingerprint,
        signature_base64: body.signature_base64,
      });
  }
  await wait(call);
  return mode.status
    ? reply(route, { error: { code: 'service_busy' } }, mode.status)
    : reply(route, result, proposal.expected_participant_revision ? 200 : 201);
}
