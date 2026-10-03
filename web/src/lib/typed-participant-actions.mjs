import { reviewValues, validateSupportSet } from './typed-participant-values.mjs';
import { proposalRequest, resolveCandidate } from './typed-participant-proposal.mjs';
import {
  base64Bytes,
  bytesBase64,
  readSubmission,
  typedParticipantFailure,
} from './typed-participant-preparation.mjs';

export function typedParticipantActions({
  get,
  patch,
  work,
  api,
  manualApi,
  credential,
  finish,
  adopt,
  onobserved,
}) {
  function reviewIdentity() {
    const state = get();
    if (state.closed || state.uncertain || state.conflict) return;
    return work(async (valid) => {
      const certificate =
        state.natural && state.value?.certificate
          ? await bytesBase64(state.value.certificate.blob)
          : null;
      if (!valid()) return;
      const review = await api.review(proposalRequest(state, certificate));
      if (valid()) patch({ review, prepared: null, preparation: null });
    });
  }
  function choose(row) {
    return work(async (valid) => {
      const result = await resolveCandidate(row, { api, manualApi, ...get() });
      if (!valid()) return;
      adopt(result.original, result.selected);
      patch({
        error: 'Candidato consultado. Revisa de nuevo la identidad con esta selecci\u00f3n.',
      });
    });
  }
  function prepare() {
    const state = get();
    if (state.closed || state.uncertain || state.conflict || !state.review) return;
    return work(async (valid) => {
      const identityReview = reviewValues(state.review, state.reason, state.decisions);
      validateSupportSet(state.review.proposal, state.selected?.values, identityReview);
      const certificate =
        state.natural && state.value?.certificate
          ? await bytesBase64(state.value.certificate.blob)
          : null;
      if (!valid()) return;
      const request = {
        proposal: state.review.proposal,
        review: identityReview,
        certificate_base64: certificate,
      };
      const ticket = certificate ? credential().beginPreparation() : null;
      try {
        const prepared = await api.prepare(request);
        if (!valid()) {
          if (ticket !== null)
            credential()?.failPreparation(ticket, 'Vuelve a preparar la declaraci\u00f3n.');
          return;
        }
        if (
          prepared.declaration &&
          !credential().acceptPreparation(
            ticket,
            base64Bytes(prepared.declaration.bytes_base64),
            new TextEncoder().encode(JSON.stringify(prepared, null, 2)),
          )
        )
          return;
        patch({ prepared, preparation: request });
      } catch (failure) {
        if (ticket !== null)
          credential()?.failPreparation(ticket, typedParticipantFailure(failure));
        throw failure;
      }
    });
  }
  async function send(bundle, valid) {
    if (!valid() || get().closed) return;
    patch({ uncertain: true, checkedAbsent: false });
    try {
      const record = await api.commit(bundle.request);
      if (valid()) await finish(record);
    } catch (failure) {
      if (valid())
        patch({ uncertain: !failure.status || failure.status >= 500, checkedAbsent: false });
      throw failure;
    }
  }
  function submit() {
    const state = get();
    if (
      state.closed ||
      state.uncertain ||
      state.conflict ||
      state.exhausted ||
      !state.prepared ||
      (state.prepared.declaration && !state.value?.ready)
    )
      return;
    return work(async (valid) => {
      const signature_base64 = state.prepared.declaration
        ? await bytesBase64(state.value.signature.blob)
        : null;
      if (!valid()) return;
      const lastSubmission = {
        prepared: state.prepared,
        request: { prepared: state.preparation, signature_base64 },
      };
      patch({ lastSubmission, uncertain: true });
      await send(lastSubmission, valid);
    });
  }
  function reconcile() {
    if (!get().lastSubmission) return;
    return work(async (valid) => {
      patch({ checkedAbsent: false });
      const result = await readSubmission(api, get().lastSubmission, valid);
      if (!valid()) return;
      if (result.state === 'absent') {
        patch({
          checkedAbsent: true,
          error:
            'Esta consulta no encontr\u00f3 la revisi\u00f3n enviada. Puedes consultar de nuevo o reenviar expl\u00edcitamente el mismo registro, sin crear otros identificadores.',
        });
      } else if (result.state === 'matched') await finish(result.record);
      else
        patch({
          current: { record: result.record, exact: true },
          error:
            'La revisi\u00f3n consultada no corresponde al env\u00edo conservado. Compara los datos; no se reenviar\u00e1 autom\u00e1ticamente.',
        });
    });
  }
  function refresh() {
    return work(async (valid) => {
      const { original, selected } = get();
      const record = original ? await manualApi.get(original.id) : null;
      if (!valid()) return;
      const bound = record?.profile ? record.subject : selected;
      const identity = bound
        ? record?.profile || original?.profile
          ? await api.subjectRevision(bound.id, bound.revision)
          : await api.subject(bound.id)
        : null;
      if (!valid()) return;
      patch({
        current: { record, identity },
        typedCurrent: get().intent === 'complete' && !!record?.profile,
      });
      if (record) await onobserved(record);
    });
  }
  function useCurrent() {
    const state = get();
    if (state.closed || !state.current || state.current.exact) return;
    adopt(
      state.current.record ?? state.original,
      state.current.identity ??
        (state.current.record?.profile ? state.current.record.subject : state.selected),
    );
    patch({ conflict: false, uncertain: false, current: null, lastSubmission: null });
  }
  return {
    reviewIdentity,
    choose,
    prepare,
    submit,
    reconcile,
    refresh,
    useCurrent,
    resend: () =>
      get().checkedAbsent &&
      get().uncertain &&
      !get().closed &&
      work((valid) => send(get().lastSubmission, valid)),
  };
}
