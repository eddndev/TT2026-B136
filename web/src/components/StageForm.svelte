<script>
  import { onDestroy } from 'svelte';
  import StageFields from './StageFields.svelte';
  import StageValues from './StageValues.svelte';
  import StageEntry from './StageEntry.svelte';
  import {
    stageDraft,
    stagePayload,
    stageAction,
    stageLabels,
    uncertainStage,
    stageSupportFields,
  } from '../lib/case-stages.mjs';
  export let api, documents, caseId, current, onconfirmed, onobserved, ondenied;
  export let oncancel,
    disabled = false,
    pending = false;
  let base = current,
    draft = stageDraft(current),
    action = stageAction(current);
  let preview = null,
    lastPayload = null,
    lastSupports = [],
    candidate,
    compared = [];
  let busy = false,
    fieldsBusy = false,
    alive = true,
    error = '',
    uncertain = false;
  let needsReview = false,
    exhausted = false,
    supportIssue = false,
    oldSupports = [];
  $: pending = busy || fieldsBusy;
  $: supportUnreviewed = supportIssue && oldSupports.some(([key, value]) => draft[key] === value);
  const refs = () =>
    Object.keys(stageSupportFields).flatMap((key) => (draft[key] ? [draft[key]] : []));
  function review() {
    if (disabled || pending || needsReview || exhausted || supportUnreviewed) return;
    try {
      preview = stagePayload(draft, base);
      error = '';
    } catch (failure) {
      error = failure.message;
    }
  }
  async function submit() {
    if (!preview || disabled || pending || needsReview || exhausted) return;
    busy = true;
    error = '';
    lastPayload = structuredClone(preview);
    lastSupports = structuredClone(refs());
    try {
      const result = await (action === 'adoption'
        ? api.adopt(lastPayload)
        : api.transition(lastPayload));
      if (!alive) return;
      await onconfirmed(result);
    } catch (failure) {
      if (!alive) return;
      if ([403, 404].includes(failure.status)) {
        ondenied(failure);
        return;
      }
      preview = null;
      uncertain = uncertainStage(failure);
      exhausted = failure.code === 'case_stage_revision_exhausted';
      needsReview =
        uncertain ||
        ['case_stage_conflict', 'case_stage_required', 'case_stage_transition_rejected'].includes(
          failure.code,
        );
      candidate = undefined;
      compared = [];
      if (['stage_support_changed', 'stage_support_digest_mismatch'].includes(failure.code)) {
        supportIssue = true;
        oldSupports = Object.keys(stageSupportFields).flatMap((key) =>
          draft[key] ? [[key, draft[key]]] : [],
        );
      }
      const uploaded = refs().some((record) => record.uploaded);
      error = uncertain
        ? `${uploaded ? 'El documento se guard\u00f3. ' : ''}No se pudo confirmar el registro. Consulta etapa e historial antes de enviar de nuevo.`
        : `${uploaded ? 'El documento se guard\u00f3. La etapa no se registr\u00f3. ' : ''}${failure.message}`;
    } finally {
      if (alive) busy = false;
    }
  }
  async function reconcile() {
    if (pending || disabled || exhausted) return;
    busy = true;
    error = '';
    try {
      const result = await api.get();
      if (!alive) return;
      onobserved(result);
      candidate = result.current;
      const history = await api.history();
      if (!alive) return;
      compared = history.entries;
    } catch (failure) {
      if (!alive) return;
      candidate = undefined;
      error = failure.message;
      if ([403, 404].includes(failure.status)) ondenied(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (candidate === undefined || pending || disabled || stageAction(candidate) !== action) return;
    base = candidate;
    needsReview = false;
    uncertain = false;
    candidate = undefined;
    preview = null;
    error = '';
    compared = [];
  }
  onDestroy(() => {
    alive = false;
    pending = false;
  });
</script>

<section class="card case-editor stage-form" aria-busy={pending}>
  <h2>
    {action === 'adoption' ? 'Registrar etapa actual' : `Registrar paso a ${stageLabels[action]}`}
  </h2>
  <p class="hint">
    Se registran los datos que declaras y sus soportes. El sistema no certifica la procedencia
    jur&#237;dica del cambio.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if supportUnreviewed}<p class="notice">
      Vuelve a elegir y consultar cada versi&#243;n seleccionada. Se conserva el borrador y no se
      cargar&#225;n archivos de nuevo autom&#225;ticamente.
    </p>{/if}
  {#if needsReview}
    <button class="secondary" disabled={pending || disabled} onclick={reconcile}
      >Consultar etapa e historial</button
    >
    {#if lastPayload}<details class="stage-last-request">
        <summary>Consultar el &#250;ltimo env&#237;o</summary>
        <p>Revisi&#243;n esperada: {lastPayload.expected_revision}.</p>
        <StageValues values={lastPayload} supports={lastSupports} />
      </details>{/if}
    {#if candidate !== undefined}<div class="case-comparison stage-reconciliation">
        <h3>Etapa consultada</h3>
        {#if candidate}<StageEntry record={candidate} />{:else}<p>Sin etapa registrada.</p>{/if}
        {#if uncertain}<p class="notice">
            Una coincidencia no confirma que este env&#237;o se guard&#243;. Revisa los registros
            antes de decidir otro intento.
          </p>{/if}
        {#if stageAction(candidate) === action}<button
            class="secondary"
            disabled={pending || disabled}
            onclick={accept}>Usar etapa consultada y revisar borrador</button
          >
        {:else}<p class="notice">
            El avance del borrador ya no corresponde a la etapa consultada. Conserva estos datos
            antes de cerrar el formulario.
          </p>{/if}
        <details>
          <summary>Historial consultado para comparar</summary
          >{#each compared as record (record.stage_revision)}<StageEntry {record} />{/each}
        </details>
      </div>{/if}
  {/if}
  {#if preview}<div class="case-comparison stage-confirmation">
      <h3>Confirma el registro</h3>
      <p>
        {action === 'adoption'
          ? 'Adopci\u00f3n de etapa conocida'
          : `${stageLabels[base.stage]} a ${stageLabels[action]}`} / revisi&#243;n esperada {preview.expected_revision}.
      </p>
      <StageValues values={preview} supports={refs()} />
      <p>El registro quedar&#225; en el historial. Revisa los datos antes de confirmarlo.</p>
      <div class="action-row">
        <button class="secondary" disabled={pending || disabled} onclick={() => (preview = null)}
          >Volver al borrador</button
        >
        <button class="primary" disabled={pending || disabled} onclick={submit}
          >{busy
            ? 'Registrando etapa...'
            : action === 'adoption'
              ? 'Registrar etapa actual'
              : 'Registrar transici\u00f3n'}</button
        >
      </div>
    </div>
  {:else}
    <StageFields
      {action}
      bind:draft
      api={documents}
      {caseId}
      {ondenied}
      disabled={disabled || busy}
      bind:pending={fieldsBusy}
    />
    <div class="action-row">
      <button
        class="primary"
        disabled={disabled || pending || needsReview || exhausted || supportUnreviewed}
        onclick={review}>Revisar registro</button
      >
    </div>
  {/if}
  <button class="text-button" disabled={pending} onclick={oncancel}
    >Cerrar formulario de etapa</button
  >
</section>
