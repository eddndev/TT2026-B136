import { stagePayload, stageAction, stageSupportFields, uncertainStage } from './case-stages.mjs';
import {
  stageBase,
  stageSubmission,
  stageSupport,
  completeStageHistory,
} from './stage-draft-values.mjs';

export const stageReferences = (draft) =>
  Object.keys(stageSupportFields).flatMap((key) => (draft[key] ? [draft[key]] : []));

export function stageFormActions({
  get,
  patch,
  work,
  api,
  caseId,
  freshContext,
  observe,
  adopt,
  finish,
}) {
  function review() {
    const state = get();
    if (
      state.disabled ||
      state.pending ||
      state.needsReview ||
      state.exhausted ||
      state.supportUnreviewed
    )
      return;
    try {
      patch({ preview: stagePayload(state.draft, state.base), error: '' });
    } catch (failure) {
      patch({ error: failure.message });
    }
  }
  function submit() {
    const state = get();
    if (!state.preview || state.disabled || state.pending || state.needsReview || state.exhausted)
      return;
    return work(async (valid) => {
      const lastPayload = stageSubmission(state.preview);
      patch({
        lastPayload,
        lastSupports: stageReferences(state.draft).map(stageSupport),
        uncertain: true,
        needsReview: true,
        historyComplete: false,
      });
      try {
        const result = await (state.action === 'adoption'
          ? api.adopt(lastPayload)
          : api.transition(lastPayload));
        if (valid()) await finish(result);
      } catch (failure) {
        if (!valid()) return;
        const uncertain = uncertainStage(failure);
        const supportIssue = ['stage_support_changed', 'stage_support_digest_mismatch'].includes(
          failure.code,
        );
        patch({
          preview: null,
          uncertain,
          candidate: undefined,
          compared: [],
          exhausted: failure.code === 'case_stage_revision_exhausted',
          needsReview:
            uncertain ||
            [
              'case_stage_conflict',
              'case_stage_required',
              'case_stage_transition_rejected',
            ].includes(failure.code),
          supportIssue,
          oldSupports: supportIssue
            ? Object.keys(stageSupportFields).flatMap((key) =>
                state.draft[key] ? [[key, state.draft[key]]] : [],
              )
            : [],
        });
        const uploaded = stageReferences(state.draft).some((record) => record.uploaded);
        failure.message = uncertain
          ? `${uploaded ? 'El documento se guard\u00f3. ' : ''}No se pudo confirmar el registro. Consulta etapa e historial antes de enviar de nuevo.`
          : `${uploaded ? 'El documento se guard\u00f3. La etapa no se registr\u00f3. ' : ''}${failure.message}`;
        throw failure;
      }
    });
  }
  function reconcile() {
    if (get().pending || get().blocked || get().exhausted) return;
    return work(async (valid) => {
      patch({ candidate: undefined, compared: [], historyComplete: false });
      try {
        const context = await freshContext();
        if (!context || !valid()) return;
        observe(context);
        patch({ candidate: context.result.current });
        const compared = await completeStageHistory(api, caseId, valid);
        if (!compared || !valid()) return;
        const candidate = context.result.current;
        if (
          (candidate === null && compared.length) ||
          (candidate !== null &&
            (!compared.length || compared[0].stage_revision !== candidate.stage_revision))
        )
          throw new Error(
            'La etapa cambi\u00f3 durante la consulta. Consulta etapa e historial de nuevo.',
          );
        patch({ compared, historyComplete: true });
      } catch (failure) {
        if (valid()) patch({ candidate: undefined, compared: [], historyComplete: false });
        throw failure;
      }
    }, true);
  }
  function accept() {
    const state = get();
    if (
      state.candidate === undefined ||
      !state.historyComplete ||
      state.pending ||
      state.disabled ||
      stageAction(state.candidate) !== state.action
    )
      return;
    adopt(stageBase(state.candidate));
    patch({
      needsReview: false,
      uncertain: false,
      candidate: undefined,
      preview: null,
      error: '',
      compared: [],
      historyComplete: false,
      lastPayload: null,
      lastSupports: [],
    });
  }
  return { review, submit, reconcile, accept };
}
