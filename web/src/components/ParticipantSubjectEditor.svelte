<script>
  import { onDestroy } from 'svelte';
  import { caseState } from '../lib/case-state.mjs';
  import {
    subjectDraft,
    subjectValues,
    reviewValues,
    validateSupportSet,
  } from '../lib/typed-participant-values.mjs';
  import { typedParticipantFailure } from '../lib/typed-participant-preparation.mjs';
  import ParticipantSubjectFields from './ParticipantSubjectFields.svelte';
  import ParticipantSubjectSummary from './ParticipantSubjectSummary.svelte';
  import ParticipantSummary from './ParticipantSummary.svelte';
  let comparison = null;
  import ParticipantCandidates from './ParticipantCandidates.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  const administration = caseState();
  export let api,
    docs,
    caseId,
    onconfirmed,
    ondenied,
    pending = false;
  let dialog,
    original = null,
    draft,
    reviewed = null,
    reason = '',
    decisions = {},
    current = null;
  let busy = false,
    fieldsBusy = false,
    candidatesBusy = false,
    error = '',
    conflict = false,
    uncertain = false,
    exhausted = false,
    last = null,
    alive = true;
  let basis, previousBasis;
  $: basis = JSON.stringify(draft);
  $: if (basis !== previousBasis) {
    previousBasis = basis;
    reviewed = null;
  }
  $: pending = busy || fieldsBusy || candidatesBusy;
  export function open(record) {
    if ($administration.closed) return;
    original = record;
    draft = subjectDraft(record.values);
    reviewed = null;
    reason = '';
    decisions = {};
    current = null;
    comparison = null;
    error = '';
    conflict = false;
    uncertain = false;
    exhausted = false;
    last = null;
    dialog.showModal();
  }
  function release() {
    dialog.close();
    original = null;
    draft = null;
    reviewed = null;
    last = null;
  }
  function close() {
    if (!pending) release();
  }
  async function work(operation) {
    if (pending || !alive) return;
    busy = true;
    error = '';
    try {
      await operation();
    } catch (failure) {
      if (alive) {
        error =
          uncertain || failure.status || failure.code
            ? typedParticipantFailure(failure)
            : failure.message;
        conflict = failure.code === 'subject_revision_conflict';
        exhausted = failure.code?.endsWith('_revision_exhausted');
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function review() {
    if ($administration.closed || uncertain || conflict) return;
    return work(async () => {
      const values = subjectValues(draft);
      validateSupportSet(values);
      const result = await api.reviewSubject(original.id, original.revision, values);
      if (alive) {
        reviewed = result;
        decisions = {};
      }
    });
  }
  function save() {
    if ($administration.closed || uncertain || conflict || !reviewed) return;
    return work(async () => {
      const values = subjectValues(draft),
        review = reviewValues(reviewed, reason, decisions);
      validateSupportSet(values, review);
      last = { expected: original.revision, values, review };
      try {
        const result = await api.replaceSubject(original.id, last.expected, values, review);
        if (!alive) return;
        await onconfirmed(result);
        if (alive) release();
      } catch (failure) {
        if (alive) uncertain = !failure.status || failure.status >= 500;
        throw failure;
      }
    });
  }
  function refresh(currentHead = false) {
    return work(async () => {
      try {
        const result =
          uncertain && !currentHead
            ? await api.subjectRevision(original.id, last.expected + 1)
            : await api.subject(original.id);
        if (alive) {
          current = result;
          error = '';
        }
      } catch (failure) {
        if (
          uncertain &&
          !currentHead &&
          failure.status === 404 &&
          failure.code === 'subject_not_found'
        ) {
          if (alive) {
            current = null;
            error =
              'Esta consulta no encontr\u00f3 la revisi\u00f3n enviada. Conservamos el formulario; puedes consultar de nuevo o revisar la identidad actual antes de decidir.';
          }
          return;
        }
        throw failure;
      }
    });
  }
  function useCurrent() {
    original = current;
    current = null;
    uncertain = false;
    conflict = false;
    reviewed = null;
  }
  onDestroy(() => {
    alive = false;
    pending = false;
    last = null;
  });
</script>

<dialog
  class="upload-dialog participant-dialog"
  aria-busy={pending}
  bind:this={dialog}
  aria-labelledby="participant-subject-editor"
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  <div class="dialog-heading">
    <h2 id="participant-subject-editor">Editar identidad del expediente</h2>
    <button class="text-button" disabled={pending} onclick={close}>Cerrar identidad</button>
  </div>
  {#if original}<div class="stack">
      <p>
        Este cambio crea una revisi&#243;n de identidad. Las fichas ya registradas conservan la
        revisi&#243;n que ten&#237;an vinculada y sus firmas anteriores.
      </p>
      <ParticipantSubjectFields
        bind:draft
        {docs}
        {caseId}
        {ondenied}
        fixedKind
        disabled={busy || candidatesBusy || $administration.closed}
        bind:pending={fieldsBusy}
      />
      <button
        class="secondary"
        disabled={pending || uncertain || conflict || exhausted || $administration.closed}
        onclick={review}>Revisar coincidencias de identidad</button
      >
      {#if reviewed}<ParticipantCandidates
          result={reviewed}
          bind:reason
          bind:decisions
          {docs}
          {caseId}
          {ondenied}
          choiceLabel="Consultar candidato"
          onchoose={(row) =>
            work(async () => {
              const result =
                row.reference.kind === 'subject'
                  ? await api.subjectRevision(row.reference.id, row.reference.revision)
                  : await api.participantRevision(row.reference.id, row.reference.revision);
              if (alive) comparison = result;
            })}
          disabled={busy || fieldsBusy || $administration.closed}
          bind:pending={candidatesBusy}
        />{/if}
      {#if comparison}<section
          class="participant-comparison"
          aria-label="Datos del candidato consultado"
        >
          <h3>Datos del candidato consultado</h3>
          {#if comparison.values}<ParticipantSubjectSummary
              record={comparison}
            />{:else}<ParticipantSummary record={comparison} />{/if}
          <p class="hint">
            Consultar no cambia la identidad que est&#225;s editando. Registra una decisi&#243;n
            distinta con soporte si corresponde; las identidades no se fusionan.
          </p>
        </section>{/if}
      {#if error}<p class="notice error" role="alert">{error}</p>{/if}
      {#if conflict || uncertain}<button
          class="secondary"
          disabled={pending}
          onclick={() => refresh(false)}
          >{uncertain
            ? 'Consultar revisi\u00f3n enviada de identidad'
            : 'Consultar identidad actual para comparar'}</button
        >{/if}
      {#if uncertain}<button class="secondary" disabled={pending} onclick={() => refresh(true)}
          >Consultar identidad actual antes de decidir</button
        >{/if}
      {#if current}<section
          class="participant-comparison"
          aria-label="Revisi&#243;n de identidad consultada"
        >
          <h3>Revisi&#243;n de identidad consultada</h3>
          <ParticipantSubjectSummary record={current} />
          <p>
            Esta consulta no atribuye el registro a tu env&#237;o. Compara los datos con tu
            formulario conservado antes de otra edici&#243;n.
          </p>
          <button
            class="secondary"
            disabled={pending || $administration.closed}
            onclick={useCurrent}>Usar esta base y conservar el formulario de identidad</button
          >
        </section>{/if}
      <CaseClosedNotice />
      <div class="dialog-actions">
        <button class="secondary" disabled={pending} onclick={close}>Cancelar</button><button
          class="primary"
          disabled={pending ||
            !reviewed ||
            uncertain ||
            conflict ||
            exhausted ||
            $administration.closed}
          onclick={save}>Guardar revisi&#243;n de identidad</button
        >
      </div>
    </div>{/if}
</dialog>
