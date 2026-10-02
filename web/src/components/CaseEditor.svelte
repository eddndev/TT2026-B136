<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import CaseReconciliation from './CaseReconciliation.svelte';
  import CaseFields from './CaseFields.svelte';
  import CaseValues from './CaseValues.svelte';
  import { caseDraft, caseFailure, manageCase } from '../lib/case-administration.mjs';
  import { descriptorKey } from '../lib/draft-descriptor.mjs';
  import {
    caseEditorDescriptor,
    sameCaseEditor,
    validateCaseEditorDraft,
  } from '../lib/case-editor-draft.mjs';
  const sessionDrafts = getContext('session-drafts');
  export let api;
  export let record = null;
  export let complete = true;
  export let disabled = false;
  export let onconfirmed;
  export let onobserved = () => {};
  export let oncancel;
  export let ondenied = () => {};
  let draft = caseDraft(record?.administration),
    fields,
    candidate = null;
  let busy = false,
    conflict = false,
    exhausted = false,
    error = '',
    alive = true;
  let uncertain = false,
    submitted = null;
  const original = record;
  const scoped = original ? api.caseAdministration(original.id) : null;
  const principalId = sessionDrafts?.principal()?.id;
  const action = original ? (complete ? 'edit-profile' : 'edit-basic') : 'create';
  let expected = original?.administration.revision ?? null;
  const descriptor = () =>
    caseEditorDescriptor(principalId, original?.id, action, instanceId, expected);
  const identity = caseEditorDescriptor(principalId, original?.id, action, null, expected);
  const matches =
    sessionDrafts?.registry.pending().filter((entry) => sameCaseEditor(entry, identity)) ?? [];
  const saved = matches.length === 1 ? matches[0] : null;
  const instanceId = original
    ? null
    : (saved?.instanceId ?? (sessionDrafts ? crypto.randomUUID() : null));
  const editorKey = descriptorKey(descriptor());
  let registration = null,
    restoring = true,
    restoreBlocked = true,
    restored = false,
    restoredClosed = false,
    needsReview = false,
    reviewAccepted = false;
  $: blocked =
    busy ||
    restoring ||
    restoreBlocked ||
    disabled ||
    restoredClosed ||
    exhausted ||
    (needsReview && !reviewAccepted) ||
    (conflict && !candidate);
  function admitted() {
    const principal = sessionDrafts?.principal();
    return (
      alive &&
      (!sessionDrafts ||
        (principal?.id === principalId && manageCase(principal?.role) && sessionDrafts.canAdmit()))
    );
  }
  function discard() {
    sessionDrafts?.registry.closeEditor(editorKey);
    registration?.dispose();
    registration = null;
  }
  function cancel() {
    if (busy || restoring) return;
    discard();
    oncancel();
  }
  function confirmed(result) {
    if (!admitted()) return;
    uncertain = false;
    discard();
    onconfirmed(result);
  }
  function deny(failure) {
    sessionDrafts?.registry.denyContext(identity.contextId);
    restoreBlocked = true;
    ondenied(failure);
  }
  function failed(failure) {
    if (!alive) return;
    error = failure.message;
    if ([403, 404].includes(failure.status)) deny(failure);
  }
  function register() {
    if (sessionDrafts)
      registration = sessionDrafts.registry.register(descriptor(), {
        fields: ['draft', 'complete', 'expected', 'rawFields', 'uncertain', 'submitted'],
        capture: () => ({
          draft,
          complete,
          expected,
          rawFields: fields.captureDraft(),
          uncertain,
          submitted,
        }),
      });
  }
  async function authorize() {
    if (original) {
      const latest = await sessionDrafts.authorizeCase(original.id);
      if (!admitted()) return null;
      if (
        latest.id !== original.id ||
        latest.administration?.case_id !== original.id ||
        !Number.isSafeInteger(latest.administration.revision) ||
        !['active', 'closed'].includes(latest.administration.administrative_status)
      )
        throw new Error('No se pudo confirmar el expediente actual.');
      restoredClosed = latest.administration.administrative_status === 'closed';
      await onobserved(latest);
      return admitted() ? latest : null;
    }
    const principal = await api.me();
    if (!admitted()) return null;
    if (principal.id !== principalId || !manageCase(principal.role))
      throw Object.assign(new Error('Ya no tienes permiso para crear expedientes.'), {
        status: 403,
      });
    return principal;
  }
  function apply(value, latest) {
    if (!admitted()) throw new Error('La sesi\u00f3n no permite recuperar el borrador.');
    validateCaseEditorDraft(value, saved.baseRevision);
    fields.restoreDraft(value.rawFields);
    draft = value.draft;
    complete = value.complete;
    expected = value.expected;
    uncertain = value.uncertain;
    submitted = value.submitted;
    restored = true;
    needsReview = uncertain || (original && latest.administration.revision !== expected);
    candidate = original && (needsReview || restoredClosed) ? latest : null;
    conflict = !!original && needsReview;
    reviewAccepted = false;
    error = uncertain
      ? 'No se pudo confirmar el resultado anterior. Consulta los datos guardados antes de decidir.'
      : '';
    register();
  }
  async function initialize() {
    if (busy || !alive) return;
    restoring = restoreBlocked = true;
    error = '';
    try {
      if (!admitted()) return;
      if (matches.length > 1)
        throw new Error('No se pudo identificar un borrador de alta \u00fanico.');
      if (saved) {
        let latest;
        const result = await sessionDrafts.registry.restore(saved.key, {
          authorize: async (entry) => {
            try {
              if (entry.schemaVersion !== 1 || !admitted()) return false;
              latest = await authorize();
              return !!latest && admitted();
            } catch (failure) {
              failed(failure);
              throw failure;
            }
          },
          apply: (value) => apply(value, latest),
        });
        if (result.status === 'restored') restoreBlocked = false;
        if (!admitted()) return;
        if (result.status !== 'restored') {
          error ||= 'No se pudo recuperar el borrador. Vuelve a consultar el contexto.';
          return;
        }
      } else {
        if (!original && sessionDrafts && !(await authorize())) return;
        if (!admitted()) return;
        register();
      }
      restoreBlocked = false;
    } catch (failure) {
      failed(failure);
    } finally {
      if (alive) restoring = false;
    }
  }
  async function refresh() {
    if (busy || restoring || restoreBlocked || !admitted()) return;
    busy = true;
    try {
      const result = restored ? await authorize() : await scoped.get();
      if (!result || !admitted()) return;
      candidate = result;
      error = '';
      if (!restored) onobserved(result);
      if (needsReview) reviewAccepted = false;
      if (result.administration.profile) complete = true;
    } catch (failure) {
      failed(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  async function submit(event) {
    event.preventDefault();
    if (blocked || !admitted()) return;
    let values;
    try {
      values = fields.values();
    } catch {
      error = '';
      return;
    }
    busy = true;
    error = '';
    uncertain = true;
    submitted = values.profile
      ? { nuc: values.profile.nuc, judicial_case_number: values.profile.judicial_case_number }
      : null;
    try {
      const result = original
        ? await scoped.replace(candidate?.administration.revision ?? expected, values)
        : await api.createPenalCase(values);
      confirmed(result);
    } catch (failure) {
      if (!alive) return;
      uncertain = !failure.status;
      conflict =
        ['case_revision_conflict', 'case_profile_required'].includes(failure.code) ||
        (restored && !!original);
      if (restored) {
        needsReview = uncertain || !!original;
        reviewAccepted = false;
      }
      exhausted = failure.code === 'case_revision_exhausted';
      candidate = null;
      error = caseFailure(failure);
      if ([403, 404].includes(failure.status)) deny(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  onMount(() => {
    initialize();
  });
  onDestroy(() => {
    alive = false;
    registration?.dispose();
    scoped?.dispose();
  });
</script>

<section
  class="card case-editor"
  aria-label="Formulario del expediente"
  aria-busy={busy || restoring}
>
  <h2>
    {!original
      ? 'Nuevo expediente penal'
      : complete
        ? 'Ficha penal'
        : 'Datos b\u00e1sicos del expediente'}
  </h2>
  <form class="stack" onsubmit={submit}>
    <CaseFields
      bind:this={fields}
      bind:draft
      {complete}
      disabled={busy || restoring || restoreBlocked}
    />
    {#if !original}<p class="notice">
        Se crear&#225; un expediente activo con etapa registrada Investigaci&#243;n.
      </p>{/if}
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if restoring}<p class="notice" role="status">Comprobando el acceso actual...</p>{/if}
    {#if restoreBlocked && !restoring}<button
        type="button"
        class="secondary"
        disabled={busy}
        onclick={initialize}>Volver a consultar el contexto</button
      >{/if}
    {#if original && (conflict || uncertain || restoredClosed)}<button
        type="button"
        class="secondary"
        disabled={busy || restoring || restoreBlocked}
        onclick={refresh}>Consultar datos actuales</button
      >{/if}
    {#if !original && uncertain && submitted && !restoring && !restoreBlocked}<CaseReconciliation
        {api}
        {submitted}
        onselect={confirmed}
      />{/if}
    {#if candidate}<section class="case-comparison" aria-label="Valores actuales guardados">
        <h3>Valores actuales guardados</h3>
        <p class="hint">Revisi&#243;n {candidate.administration.revision}</p>
        <CaseValues record={candidate.administration} />
        <p class="hint">
          Guardar mis cambios reemplazar&#225; los datos b&#225;sicos y la ficha por tu formulario.
          El estado y la etapa se conservan.
        </p>
        {#if needsReview}<label class="check-row"
            ><input
              type="checkbox"
              bind:checked={reviewAccepted}
              disabled={busy || restoring || restoreBlocked || disabled || restoredClosed}
            />
            He comparado los valores actuales y decido guardar mis cambios.
          </label>{/if}
      </section>{/if}
    {#if !original && needsReview && !restoring && !restoreBlocked}<label class="check-row">
        <input type="checkbox" bind:checked={reviewAccepted} disabled={busy} />
        He consultado los expedientes guardados y decido intentar el alta de nuevo.
      </label>{/if}
    {#if disabled || restoredClosed}<p class="notice">
        El expediente est&#225; cerrado administrativamente. Tu formulario se conserva; reactiva el
        expediente para guardar.
      </p>{/if}
    <div class="action-row">
      <button type="button" class="secondary" disabled={busy || restoring} onclick={cancel}
        >Cancelar</button
      >
      <button class="primary" disabled={blocked}
        >{restoring
          ? 'Comprobando acceso...'
          : busy
            ? 'Guardando...'
            : candidate
              ? 'Guardar mis cambios'
              : !original
                ? 'Crear expediente penal'
                : complete
                  ? 'Guardar ficha penal'
                  : 'Guardar datos b\u00e1sicos'}</button
      >
    </div>
  </form>
</section>
