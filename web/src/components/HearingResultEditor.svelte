<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import HearingResultEditorBody from './HearingResultEditorBody.svelte';
  import {
    createHearingDraft,
    refreshHearingReferences,
    resultDraftIntent,
    readResultDraftContext,
    readResultDraftSources,
  } from '../lib/hearing-draft.mjs';
  import { hearingDraftFailure } from '../lib/hearing-draft-failure.mjs';
  import { captureHearingDraft } from '../lib/hearing-draft-values.mjs';
  import { caseState } from '../lib/case-state.mjs';
  import { hearingResultDraft, hearingResultCommand } from '../lib/hearing-result-values.mjs';
  import { hearingResultFailure } from '../lib/hearing-result-errors.mjs';
  import { readHearingResultSubmission } from '../lib/hearing-result-submission.mjs';
  import { hearingDenied } from '../lib/hearings.mjs';
  export let api,
    caseId,
    user,
    hearing,
    action,
    record = null,
    continuation = null,
    ondenied,
    onconfirmed,
    oncancel,
    disabled = false,
    pending = false,
    savedDraft = null;
  const administration = caseState(),
    session = getContext('session-drafts');
  let resultId = record?.id || crypto.randomUUID();
  const hearings = api.caseHearings(caseId),
    participants = api.caseParticipants(caseId),
    typed = api.caseTypedParticipants(caseId),
    documents = api.caseDocuments(caseId);
  const anchorOf = (row) => ({
    hearing_id: row.id,
    revision: row.revision,
    values_digest: row.values_digest,
    submission_digest: row.receipt.submission_digest,
    status: row.status,
    kind: row.values.kind,
    scheduled_at: row.values.scheduled_at,
    scheduling_context: row.scheduling_context,
  });
  let anchor = record?.anchor || anchorOf(hearing),
    base = record,
    draft = hearingResultDraft(record),
    rows = structuredClone(record?.attendees || []);
  let candidates = hearing.participants,
    source = record?.continuation || continuation;
  let scoped = api.caseHearingResults(caseId, anchor.hearing_id),
    alive = true,
    busy = false,
    fieldsBusy = false,
    pickerBusy = false,
    picking = false;
  let mode = 'draft',
    prepared = null,
    last = null,
    candidate = null,
    compared = false,
    error = '';
  let blocked = true,
    restoredClosed = false,
    loaded = false,
    finished = false,
    selectors = null,
    view;
  const recovery = session
    ? createHearingDraft({
        session,
        caseId,
        result: true,
        intent: savedDraft?.instanceId ?? resultDraftIntent(hearing.id, continuation),
        capture: () =>
          captureHearingDraft(
            {
              id: resultId,
              action,
              base,
              draft,
              anchor,
              source,
              mode,
              last,
              selectors: view?.captureSelectors() ?? selectors,
            },
            true,
          ),
        read: () => readResultDraftContext(hearings, scoped, hearing.id, base),
      })
    : null;
  function admitted() {
    return alive && !finished && (!recovery || recovery.admitted());
  }
  $: pending = busy || fieldsBusy || pickerBusy;
  $: frozen =
    disabled ||
    pending ||
    blocked ||
    restoredClosed ||
    !admitted() ||
    $administration.closed ||
    ['uncertain', 'exhausted'].includes(mode);
  function observe(fresh) {
    restoredClosed = fresh.closed;
    if (mode !== 'uncertain' && (fresh.current?.revision ?? 0) !== (base?.revision ?? 0))
      mode = 'conflict';
  }
  async function initialize() {
    if (pending || !admitted()) return;
    blocked = busy = true;
    error = '';
    try {
      let fresh;
      if (savedDraft && !loaded && recovery) {
        const result = await recovery.restore(savedDraft, (value, current) => {
          ({ id: resultId, action, base, draft, anchor, source, mode, last, selectors } = value);
          scoped.dispose();
          scoped = api.caseHearingResults(caseId, anchor.hearing_id);
          loaded = true;
          fresh = current;
        });
        if (!admitted()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar el borrador de resultado.');
      } else {
        fresh = recovery
          ? await recovery.fresh()
          : { current: base, closed: $administration.closed };
        if (!fresh || !admitted()) return;
        if (!loaded) {
          recovery?.register(action, base?.id ?? null, base?.revision ?? 0);
          loaded = true;
        }
      }
      observe(fresh);
      const sources = await readResultDraftSources(api, hearings, caseId, anchor, source, admitted);
      if (!sources || !admitted()) return;
      candidates = sources.participants;
      const selected = await refreshHearingReferences(
        draft,
        typed,
        documents,
        caseId,
        admitted,
        true,
      );
      if (!selected || !admitted()) return;
      rows = selected;
      blocked = false;
    } catch (failure) {
      if (admitted()) {
        error = hearingResultFailure(failure);
        if (failure.draftReference === 'support' && [403, 404].includes(failure.status))
          supportDenied(failure);
        else if (failure.draftReference === 'owner' || hearingDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function deny(failure) {
    recovery?.deny(failure);
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    ondenied(failure);
  }
  function supportDenied(failure) {
    if (failure.code === 'case_not_found') return deny(failure);
    error = failure.message;
    recovery?.discardSupport(['provenance', 'support']);
    draft.provenance.support = null;
  }
  function supportContext() {
    return blocked
      ? null
      : (recovery?.supportContext(['provenance', 'support'], () => !blocked, observe, deny) ??
          null);
  }
  function discardSupport() {
    recovery?.discardSupport(['provenance', 'support']);
  }
  function close() {
    if (pending) return;
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    recovery?.close();
    finished = true;
    oncancel();
  }
  function fail(failure, writing = false) {
    if (hearingDenied(failure)) return deny(failure);
    ({ error, mode } = hearingDraftFailure(failure, writing, true));
    prepared = null;
    if (mode === 'conflict') compared = false;
    if (failure.code === 'case_closed') restoredClosed = true;
  }
  function selectAnchor(value) {
    if (!admitted() || blocked) return;
    anchor = anchorOf(value);
    candidates = value.participants;
    picking = false;
    scoped.dispose();
    scoped = api.caseHearingResults(caseId, anchor.hearing_id);
    prepared = null;
  }
  async function prepare() {
    if (!admitted() || frozen || mode !== 'draft') return;
    busy = true;
    error = '';
    try {
      const command = hearingResultCommand(draft, {
        action,
        base,
        operationId: crypto.randomUUID(),
        hearingId: anchor.hearing_id,
        resultId,
        anchorRevision: anchor.revision,
        continuation: source,
      });
      const value = await scoped.prepare(command, user.id);
      if (admitted()) {
        prepared = structuredClone(value);
        mode = 'review';
      }
    } catch (failure) {
      if (admitted()) fail(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  async function finish(value, exact = false) {
    recovery?.close();
    finished = true;
    prepared = null;
    mode = 'confirmed';
    try {
      await onconfirmed(value, exact);
    } catch {
      if (alive)
        error =
          'El registro se guard\u00f3. No se actualizaron todas las consultas; vuelve a consultar sin reenviar.';
    }
  }
  async function submit() {
    if (!admitted() || frozen || !prepared || mode !== 'review') return;
    busy = true;
    error = '';
    last = structuredClone(prepared);
    mode = 'uncertain';
    try {
      const value = await scoped.submit(last);
      if (admitted()) await finish(value);
    } catch (failure) {
      if (admitted()) fail(failure, true);
    } finally {
      if (alive) busy = false;
    }
  }
  async function check() {
    if (pending || blocked || !admitted() || !last) return;
    busy = true;
    error = '';
    try {
      const value = await readHearingResultSubmission(scoped, last);
      if (!admitted()) return;
      if (value.state === 'matched') await finish(value.record, true);
      else if (value.state === 'absent')
        error =
          'La revisi\u00f3n a\u00fan no est\u00e1 disponible. El resultado sigue incierto; puedes consultar de nuevo.';
      else {
        mode = 'conflict';
        candidate = value.record;
        compared = false;
        error = 'La revisi\u00f3n corresponde a otro env\u00edo. Tu borrador se conserva.';
      }
    } catch (failure) {
      if (admitted()) {
        error = hearingResultFailure(failure);
        if (hearingDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function compare() {
    if (pending || blocked || !admitted()) return;
    busy = true;
    error = '';
    compared = false;
    try {
      const value = await scoped.get(resultId);
      if (admitted()) {
        candidate = value;
        compared = true;
      }
    } catch (failure) {
      if (admitted()) {
        error = hearingResultFailure(failure);
        if (hearingDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (
      !admitted() ||
      !compared ||
      !candidate ||
      candidate.status !== 'recorded' ||
      action === 'record' ||
      frozen
    )
      return;
    base = candidate;
    recovery?.register(action, base.id, base.revision);
    mode = 'draft';
    compared = false;
    error = '';
  }
  onMount(initialize);
  onDestroy(() => {
    alive = false;
    recovery?.dispose();
    pending = false;
    scoped.dispose();
    hearings.dispose();
    participants.dispose();
    typed.dispose();
    documents.dispose();
  });
</script>

<HearingResultEditorBody
  {action}
  bind:draft
  bind:rows
  {base}
  {anchor}
  {source}
  {candidates}
  {caseId}
  {participants}
  {typed}
  {documents}
  {hearings}
  {mode}
  {prepared}
  {last}
  {candidate}
  {compared}
  {error}
  {pending}
  {frozen}
  fieldsDisabled={disabled ||
    busy ||
    blocked ||
    restoredClosed ||
    $administration.closed ||
    ['uncertain', 'exhausted'].includes(mode)}
  bind:fieldsBusy
  bind:pickerBusy
  bind:picking
  {blocked}
  {restoredClosed}
  {selectors}
  {supportContext}
  {supportDenied}
  {discardSupport}
  ondenied={deny}
  canApply={admitted}
  {prepare}
  {submit}
  {check}
  {compare}
  {accept}
  {close}
  retry={initialize}
  back={() => {
    prepared = null;
    mode = 'draft';
  }}
  {selectAnchor}
  bind:this={view}
/>
