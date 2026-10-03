<script>
  import { getContext, onDestroy, tick } from 'svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { subjectDraft, roleDraft } from '../lib/typed-participant-values.mjs';
  import { typedParticipantFailure } from '../lib/typed-participant-preparation.mjs';
  import { createTypedDraft, refreshTypedSupports } from '../lib/typed-draft.mjs';
  import { typedCapture, subjectReference } from '../lib/typed-draft-values.mjs';
  import { typedParticipantActions } from '../lib/typed-participant-actions.mjs';
  import TypedParticipantBody from './TypedParticipantBody.svelte';
  const administration = caseState();
  const session = getContext('session-drafts');
  export let api, manualApi, docs, caseId, onconfirmed, onobserved, ondenied;
  export let ondraftchange = () => {};
  let dialog,
    body,
    value = null,
    pending = false,
    generation = 0,
    alive = true;
  let root = null,
    previousBasis,
    previousReview,
    previousCertificate,
    previousSelected;
  function empty() {
    return {
      original: null,
      subject: subjectDraft(),
      selected: null,
      selectedReference: null,
      role: roleDraft(),
      review: null,
      decisions: {},
      reason: '',
      prepared: null,
      preparation: null,
      lastSubmission: null,
      busy: false,
      error: '',
      opened: false,
      uncertain: false,
      checkedAbsent: false,
      conflict: false,
      current: null,
      exhausted: false,
      blocked: false,
      loaded: false,
      restoredClosed: false,
      blockedByCase: false,
      typedCurrent: false,
      formDraft: null,
      credentialDraft: null,
      intent: 'create',
      ownerEpoch: 0,
    };
  }
  let state = empty();
  const patch = (changes) => {
    state = { ...state, ...changes };
  };
  const recovery = session
    ? createTypedDraft({
        session,
        caseId,
        api,
        manualApi,
        capture: () =>
          typedCapture(
            state,
            state.blocked ? null : { captureDraft: () => body?.captureForm() ?? null },
            state.credentialDraft !== null
              ? null
              : { captureDraft: () => body?.captureCredential() ?? null },
          ),
      })
    : null;
  const admitted = () => alive && (!recovery || recovery.admitted());
  $: closed = state.blocked || state.restoredClosed || state.typedCurrent || $administration.closed;
  $: if (state.blockedByCase && !$administration.closed) {
    state.blockedByCase = state.restoredClosed = false;
    state.error = '';
  }
  $: natural = (state.selected?.values.kind || state.subject.kind) === 'natural_person';
  $: title =
    state.intent === 'replace'
      ? 'Editar ficha tipificada'
      : state.intent === 'complete'
        ? 'Completar perfil de participante'
        : 'Agregar participante tipificado';
  $: basis = JSON.stringify({
    subject: state.subject,
    selected: state.selected,
    role: state.role,
    base: state.original?.revision,
  });
  $: if (basis !== previousBasis) {
    previousBasis = basis;
    invalidate();
  }
  $: reviewBasis = JSON.stringify({ reason: state.reason, decisions: state.decisions });
  $: if (reviewBasis !== previousReview) {
    previousReview = reviewBasis;
    state.prepared = state.preparation = null;
  }
  $: if (value?.certificate?.blob !== previousCertificate) {
    previousCertificate = value?.certificate?.blob;
    invalidate();
  }
  $: if (state.selected !== previousSelected) {
    previousSelected = state.selected;
    if (state.selected || !state.blocked)
      state.selectedReference = subjectReference(state.selected);
  }
  function invalidate() {
    state.review = state.prepared = state.preparation = null;
  }
  export function pendingDrafts() {
    return recovery?.pendingEntries() ?? [];
  }
  function entry(record, action) {
    return {
      id: record?.id ?? null,
      action: action ?? (record ? (record.profile ? 'replace' : 'complete') : 'create'),
      expected: record?.revision ?? null,
    };
  }
  export function resume(record = null) {
    const saved = pendingDrafts().find((row) => row.id === (record?.id ?? null));
    if (saved) return open(record, saved.action);
  }
  function deny(failure) {
    recovery?.discard(failure, root);
    patch({
      subject: subjectDraft(),
      selected: null,
      role: roleDraft(),
      decisions: {},
      credentialDraft: null,
      formDraft: null,
      lastSubmission: null,
      blocked: true,
    });
    ondenied(failure);
  }
  function supportDenied(failure) {
    if (!admitted()) return;
    invalidate();
    state.error = failure.message;
    if (failure.code === 'case_not_found') deny(failure);
  }
  function supportContext(path, rowId = null) {
    return state.blocked
      ? null
      : (recovery?.supportContext(path, rowId, () => state.opened && !state.blocked, deny) ?? null);
  }
  function discardPath(path, rowId) {
    recovery?.discardPath(path, rowId);
  }
  async function validateContext(context, valid) {
    patch({
      restoredClosed: context.status === 'closed',
      typedCurrent: root.action === 'complete' && !!context.record?.profile,
    });
    if (context.record && context.record.revision !== root.expected)
      patch({ current: { record: context.record }, conflict: true });
    if (
      root.action === 'replace' &&
      context.record?.revision === root.expected &&
      JSON.stringify(subjectReference(context.record.subject)) !==
        JSON.stringify(state.selectedReference)
    )
      throw new Error('La identidad vinculada no coincide con la base conservada.');
    if (state.selectedReference) {
      const selected = recovery
        ? await recovery.exactSubject(state.selectedReference)
        : await api.subjectRevision(state.selectedReference.id, state.selectedReference.revision);
      if (!valid()) return;
      patch({ selected });
    }
    const rejected = await refreshTypedSupports(state, docs, caseId, valid);
    if (!valid()) return;
    patch({ subject: state.subject, role: state.role, decisions: state.decisions });
    await tick();
    if (!valid()) return;
    if (state.credentialDraft !== null) {
      const restored = await body.restoreCredential(state.credentialDraft, valid);
      if (!valid()) return;
      if (!restored)
        throw new Error('No se pudo recuperar el material p\u00fablico de la declaraci\u00f3n.');
      patch({ credentialDraft: null });
    }
    if (!valid()) return;
    patch({
      blocked: false,
      error: rejected
        ? 'Un soporte ya no est\u00e1 disponible. Selecciona una versi\u00f3n autorizada.'
        : '',
    });
  }
  export async function open(record = null, action) {
    if (pending || !admitted() || state.opened) return;
    root = entry(record, action);
    const saved = recovery?.pending(root);
    if ($administration.closed && !saved) return;
    const ticket = ++generation,
      valid = () => admitted() && ticket === generation;
    state = { ...empty(), intent: root.action, opened: true, blocked: true, busy: true };
    await tick();
    if (!valid()) {
      if (alive && ticket === generation) state = empty();
      return;
    }
    dialog.showModal();
    try {
      let context;
      if (saved) {
        const result = await recovery.restore(root, (snapshot, fresh, savedRoot) => {
          root = savedRoot;
          context = fresh;
          patch({
            ...snapshot,
            original:
              root.id === null
                ? null
                : {
                    ...fresh.record,
                    revision: snapshot.expected,
                    profile: root.action === 'replace' ? fresh.record.profile : undefined,
                  },
            loaded: true,
          });
        });
        if (!valid()) return;
        if (result.status !== 'restored')
          throw new Error('No se pudo recuperar la ficha tipificada.');
      } else {
        context = recovery
          ? await recovery.freshContext(root)
          : { record, status: $administration.closed ? 'closed' : 'active' };
        if (!valid() || !context) return;
        root = { ...root, expected: context.record?.revision ?? null };
        patch({
          original: context.record,
          role: roleDraft('', context.record?.profile ? context.record : undefined),
          selectedReference: subjectReference(context.record?.subject),
          loaded: true,
        });
        recovery?.register(root);
      }
      await validateContext(context, valid);
    } catch (failure) {
      if (valid()) {
        state.error = failure.message;
        if ([403, 404].includes(failure.status) && !failure.documentSupportDenied) deny(failure);
      }
    } finally {
      if (alive && ticket === generation) state.busy = false;
    }
  }
  async function retry() {
    if (pending || !admitted()) return;
    if (!state.loaded) {
      state.opened = false;
      return open(root?.id ? { id: root.id, revision: root.expected } : null, root.action);
    }
    const ticket = generation,
      valid = () => admitted() && ticket === generation;
    patch({ busy: true, blocked: true, error: '' });
    try {
      const context = await recovery.freshContext(root);
      if (valid() && context) await validateContext(context, valid);
    } catch (failure) {
      if (valid()) {
        state.error = failure.message;
        if ([403, 404].includes(failure.status) && !failure.documentSupportDenied) deny(failure);
      }
    } finally {
      if (alive && ticket === generation) state.busy = false;
    }
  }
  function release() {
    recovery?.close(root);
    generation++;
    dialog.close();
    state = empty();
    pending = false;
    root = null;
    value = null;
    ondraftchange();
  }
  export function close() {
    if (!pending) release();
  }
  async function work(operation) {
    if (pending || !admitted() || state.blocked) return;
    const ticket = generation,
      valid = () => admitted() && ticket === generation;
    patch({ busy: true, error: '' });
    try {
      await operation(valid);
    } catch (failure) {
      if (valid()) {
        patch({
          error:
            state.uncertain || failure.status || failure.code
              ? typedParticipantFailure(failure)
              : failure.message,
          exhausted: failure.code?.endsWith('_revision_exhausted'),
          conflict:
            state.conflict ||
            ['participant_revision_conflict', 'subject_revision_conflict'].includes(failure.code),
        });
        if (failure.code === 'case_closed') state.blockedByCase = state.restoredClosed = true;
        if (
          [403, 404].includes(failure.status) &&
          !['subject_not_found', 'participant_credential_not_found'].includes(failure.code)
        )
          deny(failure);
      }
    } finally {
      if (alive && ticket === generation) state.busy = false;
    }
  }
  function adopt(original, selected) {
    const next = entry(original);
    const changed = next.id !== root.id || next.action !== root.action;
    if (changed) recovery?.close(root);
    root = next;
    patch({
      original,
      selected,
      selectedReference: subjectReference(selected),
      intent: root.action,
      ownerEpoch: state.ownerEpoch + (changed ? 1 : 0),
    });
    recovery?.register(root);
    invalidate();
  }
  const actions = typedParticipantActions({
    api,
    manualApi,
    work,
    patch,
    adopt,
    onobserved,
    get: () => ({ ...state, value, closed, natural }),
    credential: () => body?.credentialControl(),
    finish: async (record) => {
      release();
      await onconfirmed(record);
    },
  });
  onDestroy(() => {
    alive = false;
    generation++;
    recovery?.dispose();
  });
</script>

<dialog
  class="upload-dialog participant-dialog"
  aria-busy={pending}
  bind:this={dialog}
  aria-labelledby="typed-participant-title"
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  <div class="dialog-heading">
    <div>
      <span class="eyebrow">DIRECTORIO DEL EXPEDIENTE</span>
      <h2 id="typed-participant-title">{title}</h2>
    </div>
    <button class="text-button" disabled={pending} onclick={close}>Cerrar ficha</button>
  </div>
  {#if state.opened}<TypedParticipantBody
      bind:this={body}
      bind:state
      bind:value
      bind:pending
      {actions}
      {api}
      {manualApi}
      {docs}
      {caseId}
      {generation}
      {closed}
      {admitted}
      {supportContext}
      {discardPath}
      ondenied={supportDenied}
      {close}
      {retry}
    />{/if}
</dialog>
