<script>
  import { onDestroy } from 'svelte';
  import CalendarFields from './CalendarFields.svelte';
  import CalendarValues from './CalendarValues.svelte';
  import { calendarDraft, calendarCommand } from '../lib/judicial-calendar-values.mjs';
  import {
    calendarDenied,
    calendarUncertain,
    calendarFailure,
  } from '../lib/judicial-calendar-labels.mjs';
  import { readCalendarSubmission } from '../lib/judicial-calendar-submission.mjs';
  export let api,
    user,
    action,
    record = null,
    onconfirmed,
    oncancel,
    ondenied,
    busy = false;
  const scoped = api.judicialCalendars(),
    calendarId = record?.id || crypto.randomUUID();
  let base = record,
    draft = calendarDraft(record),
    prepared = null,
    last = null,
    candidate = null,
    compared = false,
    mode = 'draft',
    error = '',
    alive = true;
  $: frozen = busy || ['uncertain', 'exhausted', 'confirmed'].includes(mode);
  function fail(failure, writing = false) {
    if (calendarDenied(failure)) {
      ondenied(failure);
      return;
    }
    error = calendarFailure(failure);
    prepared = null;
    if (
      writing &&
      (calendarUncertain(failure) || failure.code === 'judicial_calendar_operation_conflict')
    ) {
      mode = 'uncertain';
      error =
        'No se pudo confirmar el resultado. Conservamos el env\u00edo para consultar su revisi\u00f3n exacta.';
    } else if (failure.code === 'judicial_calendar_revision_exhausted') mode = 'exhausted';
    else if (
      [
        'judicial_calendar_revision_conflict',
        'judicial_calendar_retired',
        'judicial_calendar_not_found',
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
      const command = calendarCommand(draft, {
        action,
        base,
        calendarId,
        operationId: crypto.randomUUID(),
      });
      const row = await scoped.prepare(command, user.id);
      if (alive) {
        prepared = structuredClone(row);
        mode = 'review';
      }
    } catch (failure) {
      if (alive) fail(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  async function finish(row, exact = false) {
    mode = 'confirmed';
    prepared = null;
    try {
      await onconfirmed(row, exact);
    } catch {
      if (alive)
        error =
          'El calendario se guard\u00f3. Vuelve a consultar la lista sin reenviar la escritura.';
    }
  }
  async function submit() {
    if (frozen || mode !== 'review' || !prepared) return;
    busy = true;
    error = '';
    last = structuredClone(prepared);
    try {
      const row = await scoped.submit(last);
      if (alive) await finish(row);
    } catch (failure) {
      if (alive) fail(failure, true);
    } finally {
      if (alive) busy = false;
    }
  }
  async function check() {
    if (busy || !last) return;
    busy = true;
    error = '';
    try {
      const result = await readCalendarSubmission(scoped, last);
      if (!alive) return;
      if (result.state === 'matched') await finish(result.record, true);
      else if (result.state === 'absent')
        error =
          'La revisi\u00f3n a\u00fan no est\u00e1 disponible. El resultado sigue incierto; puedes consultar de nuevo.';
      else {
        mode = 'conflict';
        candidate = result.record;
        compared = false;
        error = 'La revisi\u00f3n corresponde a otro env\u00edo. Tu borrador se conserva.';
      }
    } catch (failure) {
      if (alive) {
        error = calendarFailure(failure);
        if (calendarDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function compare() {
    if (busy) return;
    busy = true;
    error = '';
    compared = false;
    try {
      const row = await scoped.get(calendarId);
      if (alive) {
        candidate = row;
        compared = true;
      }
    } catch (failure) {
      if (alive) {
        error = calendarFailure(failure);
        if (calendarDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (!compared || candidate?.status !== 'published' || action === 'publish' || busy) return;
    base = candidate;
    mode = 'draft';
    compared = false;
    error = '';
  }
  onDestroy(() => {
    alive = false;
    busy = false;
    scoped.dispose();
  });
</script>

<section class="card calendar-editor" aria-label="Formulario de calendario" aria-busy={busy}>
  <h2>
    {action === 'publish'
      ? 'Publicar calendario'
      : action === 'replace'
        ? 'Reemplazar calendario'
        : 'Retirar calendario'}
  </h2>
  <p class="hint">
    Clasificaci&#243;n declarada por &#225;mbito. Esta captura no calcula plazos ni acredita el
    contenido de las fuentes.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if mode === 'uncertain'}<div class="case-comparison">
      <h3>Resultado incierto</h3>
      <p>Consulta el recibo del env&#237;o. La ausencia temporal no confirma que fall&#243;.</p>
      <button class="primary" disabled={busy} onclick={check}>Consultar env&#237;o exacto</button>
      <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
    </div>{/if}
  {#if last}<details>
      <summary>Identidad del &#250;ltimo env&#237;o</summary>
      <p>
        Calendario: <code>{last.command.calendar_id}</code> / Revisi&#243;n objetivo {last.result_revision}
      </p>
      <p>Operaci&#243;n: <code>{last.command.operation_id}</code></p>
      <p>Recibo: <code>{last.submission_digest}</code></p>
    </details>{/if}
  {#if mode === 'conflict'}<div class="case-comparison">
      <h3>Comparar con la base actual</h3>
      <p>
        Tu borrador se conserva. Consulta los valores actuales antes de preparar una nueva
        operaci&#243;n.
      </p>
      <button class="secondary" disabled={busy} onclick={compare}
        >Consultar base actual del calendario</button
      >
      {#if compared && candidate}<p>
          Revisi&#243;n {candidate.revision} / {candidate.status === 'retired'
            ? 'Retirado'
            : 'Publicado'}
        </p>
        <CalendarValues values={candidate.values} />
        {#if action !== 'publish' && candidate.status === 'published'}<button
            class="primary"
            disabled={busy}
            onclick={accept}>Usar esta base y conservar borrador</button
          >{:else}<p>
            Esta base no admite reenviar el borrador. Cierra el formulario y consulta su historia.
          </p>{/if}
      {/if}
    </div>{/if}
  {#if prepared}<div class="case-comparison">
      <h3>Revisa el calendario a registrar</h3>
      <p>Revisi&#243;n a registrar: {prepared.result_revision}</p>
      <CalendarValues values={prepared.values} />{#if prepared.command.change.reason}<p
          class="case-multiline"
        >
          Motivo: {prepared.command.change.reason}
        </p>{/if}
      <details>
        <summary>Recibo de la preparaci&#243;n</summary>
        <p>Operaci&#243;n: <code>{prepared.command.operation_id}</code></p>
        <p>Valores: <code>{prepared.values_digest}</code></p>
        <p>Recibo: <code>{prepared.submission_digest}</code></p>
      </details>
      <div class="action-row">
        <button
          class="secondary"
          disabled={busy}
          onclick={() => {
            prepared = null;
            mode = 'draft';
          }}>Volver al borrador del calendario</button
        ><button class="primary" disabled={frozen} onclick={submit}>Confirmar calendario</button>
      </div>
    </div>{:else if mode !== 'confirmed'}
    {#if action === 'retire'}<CalendarValues values={base.values} />
      <p class="notice">
        Retirar conserva todos los valores e historia. Es terminal y no declara derogaci&#243;n.
      </p>
    {:else}<CalendarFields
        bind:draft
        disabled={frozen}
        immutableScope={action !== 'publish'}
      />{/if}
    {#if action !== 'publish'}<label
        >Motivo<textarea rows="3" bind:value={draft.reason} disabled={frozen}></textarea></label
      >{/if}
    <button class="primary" disabled={frozen || mode !== 'draft'} onclick={prepare}
      >Revisar calendario</button
    >
  {/if}
  <button class="text-button" disabled={busy} onclick={oncancel}
    >Cerrar formulario de calendario</button
  >
</section>
