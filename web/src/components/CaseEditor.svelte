<script>
  import { onDestroy } from 'svelte';
  import CaseReconciliation from './CaseReconciliation.svelte';
  import CaseFields from './CaseFields.svelte';
  import CaseValues from './CaseValues.svelte';
  import { caseDraft, caseFailure } from '../lib/case-administration.mjs';
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
  async function refresh() {
    busy = true;
    try {
      const result = await scoped.get();
      if (!alive) return;
      candidate = result;
      error = '';
      onobserved(result);
      if (result.administration.profile) complete = true;
    } catch (failure) {
      if (alive) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function submit(event) {
    event.preventDefault();
    if (busy || disabled || exhausted || (conflict && !candidate)) return;
    let values;
    try {
      values = fields.values();
    } catch {
      error = '';
      return;
    }
    busy = true;
    error = '';
    uncertain = false;
    submitted = values.profile
      ? { nuc: values.profile.nuc, judicial_case_number: values.profile.judicial_case_number }
      : null;
    try {
      const result = original
        ? await scoped.replace(
            candidate?.administration.revision ?? original.administration.revision,
            values,
          )
        : await api.createPenalCase(values);
      if (alive) onconfirmed(result);
    } catch (failure) {
      if (!alive) return;
      uncertain = !failure.status;
      conflict = ['case_revision_conflict', 'case_profile_required'].includes(failure.code);
      exhausted = failure.code === 'case_revision_exhausted';
      candidate = null;
      error = caseFailure(failure);
      if ([403, 404].includes(failure.status)) ondenied(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
    scoped?.dispose();
  });
</script>

<section class="card case-editor" aria-label="Formulario del expediente">
  <h2>
    {!original
      ? 'Nuevo expediente penal'
      : complete
        ? 'Ficha penal'
        : 'Datos b\u00e1sicos del expediente'}
  </h2>
  <form class="stack" onsubmit={submit}>
    <CaseFields bind:this={fields} bind:draft {complete} disabled={busy} />
    {#if !original}<p class="notice">
        Se crear&#225; un expediente activo con etapa registrada Investigaci&#243;n.
      </p>{/if}
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if original && (conflict || uncertain)}<button
        type="button"
        class="secondary"
        disabled={busy}
        onclick={refresh}>Consultar datos actuales</button
      >{/if}
    {#if !original && uncertain && submitted}<CaseReconciliation
        {api}
        {submitted}
        onselect={onconfirmed}
      />{/if}
    {#if candidate}<section class="case-comparison" aria-label="Valores actuales guardados">
        <h3>Valores actuales guardados</h3>
        <p class="hint">Revisi&#243;n {candidate.administration.revision}</p>
        <CaseValues record={candidate.administration} />
        <p class="hint">
          Guardar mis cambios reemplazar&#225; los datos b&#225;sicos y la ficha por tu formulario.
          El estado y la etapa se conservan.
        </p>
      </section>{/if}
    {#if disabled}<p class="notice">
        El expediente est&#225; cerrado administrativamente. Tu formulario se conserva; reactiva el
        expediente para guardar.
      </p>{/if}
    <div class="action-row">
      <button type="button" class="secondary" disabled={busy} onclick={oncancel}>Cancelar</button>
      <button class="primary" disabled={busy || disabled || exhausted || (conflict && !candidate)}
        >{busy
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
