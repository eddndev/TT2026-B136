<script>
  import { onDestroy } from 'svelte';
  import FactFields from './FactFields.svelte';
  import FactValues from './FactValues.svelte';
  import FactSources from './FactSources.svelte';
  import FactAdministrativeCapture from './FactAdministrativeCapture.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import { factDraft, factCommand } from '../lib/procedural-fact-values.mjs';
  import { factFailure, factDenied, factUncertain } from '../lib/procedural-fact-errors.mjs';
  import { readFactSubmission } from '../lib/procedural-fact-submission.mjs';
  export let api,
    caseId,
    user,
    family,
    resolution = null,
    action,
    record = null,
    ondenied,
    onconfirmed,
    oncancel,
    disabled = false,
    pending = false;
  const administration = caseState(),
    id = record?.id || crypto.randomUUID();
  const scoped =
    family === 'resolution'
      ? api.caseResolutions(caseId)
      : api.caseNotifications(caseId, resolution.id);
  const singular = family === 'resolution' ? 'resoluci\u00f3n' : 'notificaci\u00f3n';
  let base = record,
    draft = factDraft(family, record, resolution),
    alive = true,
    busy = false,
    fieldsBusy = false;
  let mode = 'draft',
    prepared = null,
    last = null,
    candidate = null,
    compared = false,
    error = '';
  $: pending = busy || fieldsBusy;
  $: frozen =
    disabled || pending || $administration.closed || ['uncertain', 'exhausted'].includes(mode);
  function fail(failure, writing = false) {
    if (factDenied(failure)) {
      ondenied(failure);
      return;
    }
    error = factFailure(failure);
    prepared = null;
    if (
      writing &&
      (factUncertain(failure) || failure.code === 'procedural_fact_operation_conflict')
    ) {
      mode = 'uncertain';
      error =
        'No se pudo confirmar el resultado. Conservamos el env\u00edo para consultar su revisi\u00f3n exacta.';
    } else if (failure.code === 'procedural_fact_revision_exhausted') mode = 'exhausted';
    else if (
      [
        'procedural_fact_revision_conflict',
        'procedural_fact_already_withdrawn',
        'procedural_fact_not_found',
      ].includes(failure.code)
    ) {
      mode = 'conflict';
      compared = false;
    } else mode = 'draft';
  }
  async function prepare() {
    if (frozen || mode !== 'draft') return;
    busy = true;
    error = '';
    try {
      const command = factCommand(draft, {
        family,
        action,
        base,
        operationId: crypto.randomUUID(),
        id,
        resolutionId: resolution?.id,
      });
      const value = await scoped.prepare(command, user.id);
      if (alive) {
        prepared = structuredClone(value);
        mode = 'review';
      }
    } catch (failure) {
      if (alive) fail(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  async function finish(value, exact = false) {
    prepared = null;
    mode = 'confirmed';
    try {
      await onconfirmed(value, exact);
    } catch {
      if (alive)
        error =
          'El registro se guard\u00f3. Vuelve a consultar sin reenviar si faltan datos en pantalla.';
    }
  }
  async function submit() {
    if (frozen || !prepared || mode !== 'review') return;
    busy = true;
    error = '';
    last = structuredClone(prepared);
    try {
      const value = await scoped.submit(last);
      if (alive) await finish(value);
    } catch (failure) {
      if (alive) fail(failure, true);
    } finally {
      if (alive) busy = false;
    }
  }
  async function check() {
    if (pending || !last) return;
    busy = true;
    error = '';
    try {
      const value = await readFactSubmission(scoped, last);
      if (!alive) return;
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
      if (alive) {
        error = factFailure(failure);
        if (factDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function compare() {
    if (pending) return;
    busy = true;
    error = '';
    compared = false;
    try {
      const value = await scoped.get(id);
      if (alive) {
        candidate = value;
        compared = true;
      }
    } catch (failure) {
      if (alive) {
        error = factFailure(failure);
        if (factDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (!compared || !candidate || candidate.status !== 'recorded' || action === 'record' || frozen)
      return;
    base = candidate;
    mode = 'draft';
    compared = false;
    error = '';
  }
  onDestroy(() => {
    alive = false;
    pending = false;
    scoped.dispose();
  });
</script>

<section
  class="card case-editor fact-editor"
  aria-label={`Formulario de ${singular}`}
  aria-busy={pending}
>
  <h2>
    {action === 'record' ? 'Registrar' : action === 'correct' ? 'Corregir' : 'Retirar'}
    {singular}
  </h2>
  <p class="hint">
    Registra lo declarado con sus fuentes. La captura no determina efectos jur&#237;dicos ni inicia
    plazos.
  </p>
  <CaseClosedNotice />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if mode === 'uncertain'}<div class="case-comparison">
      <h3>Resultado incierto</h3>
      <p>Consulta el recibo del env&#237;o. Una ausencia temporal no confirma que fall&#243;.</p>
      <button class="primary" disabled={pending} onclick={check}>Consultar env&#237;o exacto</button
      >
      <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
    </div>{/if}
  {#if mode === 'conflict'}<div class="case-comparison">
      <h3>Comparar con el registro actual</h3>
      <button class="secondary" disabled={pending} onclick={compare}>Consultar base actual</button>
      {#if compared && candidate}<p>
          Base consultada: revisi&#243;n {candidate.revision} / {candidate.status === 'withdrawn'
            ? 'Retirado'
            : 'Registrado'}
        </p>
        <FactValues values={candidate.values} {family} /><FactSources sources={candidate.sources} />
        {#if action !== 'record' && candidate.status === 'recorded'}<button
            class="primary"
            disabled={frozen}
            onclick={accept}>Usar esta base y conservar borrador</button
          >
        {:else}<p>
            Esta captura no permite reenviar sobre la base consultada. Puedes cerrar y revisar sus
            fuentes.
          </p>{/if}
      {/if}
    </div>{/if}
  {#if prepared}<div class="case-comparison">
      <h3>Revisa el registro a confirmar</h3>
      <p>Revisi&#243;n a registrar: {prepared.result_revision}</p>
      <FactValues values={prepared.values} {family} /><FactSources
        sources={prepared.sources}
      /><FactAdministrativeCapture value={prepared.observed_administration} />
      {#if prepared.command.change.reason}<p class="case-multiline">
          Motivo: {prepared.command.change.reason}
        </p>{/if}
      <div class="action-row">
        <button
          class="secondary"
          disabled={pending}
          onclick={() => {
            prepared = null;
            mode = 'draft';
          }}>Volver al borrador</button
        ><button class="primary" disabled={frozen} onclick={submit}>Confirmar registro</button>
      </div>
    </div>{:else if mode !== 'confirmed'}
    {#if action === 'withdraw'}<FactValues values={base.values} {family} /><FactSources
        sources={base.sources}
      />
      <p class="notice">Retirar conserva la captura y su historia; no anula el acto declarado.</p>
    {:else}<FactFields
        bind:values={draft.values}
        {family}
        {api}
        {caseId}
        {resolution}
        {ondenied}
        disabled={disabled ||
          busy ||
          $administration.closed ||
          ['uncertain', 'exhausted'].includes(mode)}
        bind:pending={fieldsBusy}
      />{/if}
    {#if action !== 'record'}<label
        >Motivo<textarea rows="3" bind:value={draft.reason} disabled={frozen}></textarea></label
      >{/if}
    <button class="primary" disabled={frozen || mode !== 'draft'} onclick={prepare}
      >Preparar registro</button
    >
  {/if}
  <button class="text-button" disabled={pending} onclick={oncancel}>Cerrar formulario</button>
</section>
