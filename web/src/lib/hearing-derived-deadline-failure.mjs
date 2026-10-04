import { hearingResultFailure } from './hearing-result-errors.mjs';
import { deadlineFailure } from './deadline-errors.mjs';

export function createHearingDerivedDeadlineFailure({ read, update, discardSupport, deny }) {
  return (failure, writing = false) => {
    const support = failure.draftReference === 'support';
    if (
      failure.code === 'case_not_found' ||
      failure.draftReference === 'owner' ||
      (failure.status === 403 && !support)
    ) {
      deny(failure);
      return;
    }
    const value = read();
    const error =
      failure.status === 413 || failure.code?.startsWith('deadline_')
        ? deadlineFailure(failure)
        : hearingResultFailure(failure);
    if (support && [403, 404].includes(failure.status) && !value.last) {
      const draft = structuredClone(value.draft);
      draft.provenance.support = null;
      discardSupport();
      update({
        draft,
        mode: 'draft',
        prepared: null,
        acknowledged: false,
        retryAvailable: false,
        error,
      });
      return;
    }
    update({
      error,
      retryAvailable: false,
      ...(writing && !['uncertain', 'conflict'].includes(value.mode) ? { mode: 'uncertain' } : {}),
    });
  };
}
