<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import HearingEditorBody from './HearingEditorBody.svelte';
  import {
    createHearingDraft,
    refreshHearingReferences,
    readHearingDraftContext,
  } from '../lib/hearing-draft.mjs';
  import { hearingDraftFailure } from '../lib/hearing-draft-failure.mjs';
  import { captureHearingDraft } from '../lib/hearing-draft-values.mjs';
  import { caseState } from '../lib/case-state.mjs';
  import { hearingDraft, hearingCommand, hearingKinds, hearingDenied } from '../lib/hearings.mjs';
  import { hearingFailure } from '../lib/hearing-errors.mjs';
  import { readHearingSubmission } from '../lib/hearing-submission.mjs';
  export let api,
    participantsApi,
    typedApi,
    documents,
    caseId,
    user,
    action,
    record,
    initialContext,
    ondenied,
    onconfirmed,
    oncontext,
    oncancel,
    disabled = false,
    pending = false,
    savedDraft = null;
  const administration = caseState(),
    session = getContext('session-drafts');
  let hearingId = record?.id || crypto.randomUUID();
  let base = record,
    context = initialContext,
    draft = hearingDraft(record);
  if (!record)
    draft.kind =
      Object.keys(hearingKinds).find((key) => hearingKinds[key].stage === context?.stage) ||
      'initial';
  let rows = (record?.participants || []).map((row) => ({ ...row, retained: true }));
  let prepared = null,
    last = null,
    candidate = null,
    comparedContext = null;
  let mode = 'draft',
    error = '',
    issue = '',
    comparisonReady = false,
    alive = true,
    busy = false,
    fieldsBusy = false;
  let blocked = true,
    restoredClosed = false,
    loaded = false,
    finished = false,
    view,
    selectors = null;
  const recovery = session
    ? createHearingDraft({
        session,
        caseId,
        capture: () =>
          captureHearingDraft({
            id: hearingId,
            action,
            base,
            draft,
            context,
            mode,
            last,
            selectors: view?.captureSelectors() ?? selectors,
          }),
        read: () => readHearingDraftContext(api, base),
      })
    : null;
  function admitted() {
    return alive && !finished && (!recovery || recovery.admitted());
  }
  $: pending = busy || fieldsBusy;
  $: frozen =
    disabled ||
    pending ||
    blocked ||
    restoredClosed ||
    !admitted() ||
    $administration.closed ||
    ['uncertain', 'exhausted'].includes(mode);
  $: canAccept =
    comparisonReady &&
    mode === 'conflict' &&
    comparedContext?.administrative_status === 'active' &&
    (action === 'schedule'
      ? !candidate
      : candidate?.status === 'scheduled' && candidate.values.kind === base.values.kind);
  function observe(fresh) {
    restoredClosed = fresh.closed;
    if (
      mode !== 'uncertain' &&
      ((fresh.current?.revision ?? 0) !== (base?.revision ?? 0) ||
        (action !== 'cancel' &&
          (fresh.context.case_revision !== context?.case_revision ||
            fresh.context.stage_revision !== context?.stage_revision)))
    )
      mode = 'conflict';
    oncontext(fresh.context);
  }
  async function initialize() {
    if (pending || !admitted()) return;
    blocked = busy = true;
    error = '';
    try {
      let fresh;
      if (savedDraft && !loaded && recovery) {
        const result = await recovery.restore(savedDraft, (value, current) => {
          ({ id: hearingId, action, base, draft, context, mode, last, selectors } = value);
          loaded = true;
          fresh = current;
        });
        if (!admitted()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar el borrador de audiencia.');
      } else {
        fresh = recovery
          ? await recovery.fresh()
          : { context: await api.context(), current: base, closed: $administration.closed };
        if (!fresh || !admitted()) return;
        if (!loaded) {
          recovery?.register(action, base?.id ?? null, base?.revision ?? 0);
          context = fresh.context;
          loaded = true;
        }
      }
      observe(fresh);
      const selected = await refreshHearingReferences(draft, typedApi, documents, caseId, admitted);
      if (!selected || !admitted()) return;
      rows = selected;
      blocked = false;
    } catch (failure) {
      if (admitted()) {
        error = hearingFailure(failure);
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
    issue = 'support';
    recovery?.discardSupport(['conviction_basis', 'support']);
    draft.support = null;
  }
  function supportContext() {
    return blocked
      ? null
      : (recovery?.supportContext(['conviction_basis', 'support'], () => !blocked, observe, deny) ??
          null);
  }
  function discardSupport() {
    recovery?.discardSupport(['conviction_basis', 'support']);
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
    ({ error, mode, issue } = hearingDraftFailure(failure, writing, false));
    prepared = null;
    if (mode === 'conflict') comparisonReady = false;
    if (failure.code === 'case_closed') restoredClosed = true;
  }
  async function prepare() {
    if (!admitted() || frozen || issue || mode !== 'draft') return;
    busy = true;
    error = '';
    try {
      const command = hearingCommand(draft, context, base, action, crypto.randomUUID(), hearingId);
      const result = await api.prepare(command, user.id);
      if (admitted()) {
        prepared = structuredClone(result);
        mode = 'review';
      }
    } catch (failure) {
      if (admitted()) fail(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  async function finish(result, exact = false) {
    recovery?.close();
    finished = true;
    mode = 'confirmed';
    prepared = null;
    try {
      await onconfirmed(result, exact);
    } catch {
      if (alive)
        error =
          'La audiencia se guard\u00f3. No se pudieron actualizar todas las consultas; vuelve a consultar sin reenviar.';
    }
  }
  async function submit() {
    if (!admitted() || !prepared || frozen || mode !== 'review') return;
    busy = true;
    error = '';
    last = structuredClone(prepared);
    mode = 'uncertain';
    let result;
    try {
      result = await api.submit(last);
    } catch (failure) {
      if (admitted()) fail(failure, true);
      if (alive) busy = false;
      return;
    }
    if (admitted()) {
      await finish(result);
    }
    if (alive) busy = false;
  }
  async function check() {
    if (pending || blocked || !admitted() || !last) return;
    busy = true;
    error = '';
    try {
      const result = await readHearingSubmission(api, last);
      if (!admitted()) return;
      if (result.state === 'matched') await finish(result.record, true);
      else if (result.state === 'absent')
        error =
          'La revisi\u00f3n a\u00fan no est\u00e1 disponible. El resultado sigue incierto; puedes consultar de nuevo.';
      else {
        mode = 'conflict';
        candidate = result.record;
        comparisonReady = false;
        error = 'La revisi\u00f3n pertenece a otro env\u00edo. Tu borrador se conserva.';
      }
    } catch (failure) {
      if (admitted()) {
        error = hearingFailure(failure);
        if (hearingDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function refresh() {
    if (pending || blocked || !admitted()) return;
    busy = true;
    error = '';
    comparisonReady = false;
    try {
      const nextContext = await api.context();
      let current = null;
      try {
        current = await api.get(hearingId);
      } catch (failure) {
        if (failure.code !== 'hearing_not_found' || failure.status !== 404) throw failure;
      }
      if (!admitted()) return;
      comparedContext = nextContext;
      candidate = current;
      comparisonReady = true;
      oncontext(nextContext);
    } catch (failure) {
      if (admitted()) {
        error = hearingFailure(failure);
        if (hearingDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (!admitted() || !canAccept || pending) return;
    base = candidate;
    recovery?.register(action, base?.id ?? null, base?.revision ?? 0);
    context = comparedContext;
    rows = rows.map((row) => ({
      ...row,
      retained: !!base?.values.participants.some(
        (ref) => ref.participant_id === row.id && ref.revision === row.revision,
      ),
    }));
    prepared = null;
    comparisonReady = false;
    mode = 'draft';
    error = '';
  }
  onMount(initialize);
  onDestroy(() => {
    alive = false;
    recovery?.dispose();
    pending = false;
  });
</script>

<HearingEditorBody
  {action}
  bind:draft
  bind:rows
  {base}
  {context}
  {participantsApi}
  {typedApi}
  {documents}
  {caseId}
  {mode}
  {prepared}
  {candidate}
  {comparedContext}
  {comparisonReady}
  {canAccept}
  {pending}
  {frozen}
  {error}
  {issue}
  {last}
  {busy}
  fieldsDisabled={disabled ||
    busy ||
    blocked ||
    restoredClosed ||
    $administration.closed ||
    ['uncertain', 'exhausted'].includes(mode)}
  bind:fieldsBusy
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
  {refresh}
  {accept}
  {close}
  retry={initialize}
  back={() => {
    prepared = null;
    mode = 'draft';
  }}
  reviewed={(kind) => {
    if (issue === kind) issue = '';
  }}
  bind:this={view}
/>
