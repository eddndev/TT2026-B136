<script>
  import { onDestroy } from 'svelte';
  import {
    alertKindLabel,
    alertEmailLabels,
    alertResolutionLabels,
    alertTimeLabel,
    alertFailure,
  } from '../lib/alerts-presentation.mjs';
  export let row,
    api,
    onopen,
    onread,
    ondenied,
    disabled = false;
  let busy = false,
    uncertain = false,
    error = '',
    operation = null,
    alive = true;
  $: resolved = row.state.kind === 'resolved';
  $: tone = resolved ? 'neutral' : row.kind.kind === 'overdue_unattended' ? 'danger' : 'warning';
  function denied(failure) {
    if ([403, 404].includes(failure.status)) ondenied(failure, row);
  }
  async function markRead() {
    if (disabled || busy || uncertain || row.read_at !== null) return;
    operation ??= { operation_id: crypto.randomUUID() };
    busy = true;
    error = '';
    try {
      const result = await api.markRead(row.id, operation);
      if (alive) onread(result.alert, result.checked_at);
    } catch (failure) {
      if (alive) {
        error = alertFailure(failure);
        uncertain = ![400, 403, 404, 409, 422].includes(failure.status);
        denied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function checkRead() {
    if (disabled || busy) return;
    busy = true;
    error = '';
    try {
      const result = await api.get(row.id);
      if (!alive) return;
      uncertain = false;
      if (result.alert.read_at !== null) onread(result.alert, result.checked_at);
      else
        error =
          'La consulta no confirma la lectura. Puedes marcarla de nuevo con la misma operacion.';
    } catch (failure) {
      if (alive) {
        error = alertFailure(failure);
        denied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
  });
</script>

<article class="alert-card" data-alert-id={row.id} aria-label={`Alerta ${row.id}`} aria-busy={busy}>
  <div class="alert-card-heading">
    <span class={`badge ${tone}`}>{alertKindLabel(row)}</span>
    <span class={`badge ${row.read_at === null ? 'info' : 'neutral'}`}>
      {row.read_at === null ? 'Sin leer' : 'Le\u00edda'}
    </span>
  </div>
  <h2>{row.subject_title}</h2>
  <p class="alert-case">{row.case_title} <span aria-hidden="true">/</span> {row.case_reference}</p>
  {#if row.kind.kind === 'upcoming'}
    <p class="alert-reason">{row.kind.lead_hours} horas antes</p>
    <dl class="alert-times">
      <div>
        <dt>Fecha de actividad capturada</dt>
        <dd>{alertTimeLabel(row.kind.activity_at)}</dd>
      </div>
    </dl>
  {:else if row.kind.kind === 'overdue_unattended'}
    <dl class="alert-times">
      <div>
        <dt>Vencimiento capturado</dt>
        <dd>{alertTimeLabel(row.kind.due_at)}</dd>
      </div>
    </dl>
  {:else if row.kind.kind === 'due_changed_soon'}
    <dl class="alert-times">
      <div>
        <dt>Fecha anterior capturada</dt>
        <dd>{alertTimeLabel(row.kind.previous_due_at)}</dd>
      </div>
      <div>
        <dt>Fecha nueva capturada</dt>
        <dd>{alertTimeLabel(row.kind.current_due_at)}</dd>
      </div>
    </dl>
  {:else}<p>El aviso requiere revisar el plazo; no acredita una fecha operativa.</p>{/if}
  <div class="alert-provenance">
    <span>Revisi&#243;n de origen {row.origin.revision}</span>
    <span>Generada: {alertTimeLabel(row.created_at)}</span>
  </div>
  {#if resolved}<p class="alert-resolution">
      <span class="badge neutral">Resuelta</span>
      {alertResolutionLabels[row.state.reason]}
    </p>{/if}
  <p class="alert-email">
    <span class="badge neutral">{alertEmailLabels[row.email.kind]}</span>
    {#if row.email.kind === 'accepted'}<span>{alertTimeLabel(row.email.accepted_at)}</span>{/if}
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  <div class="action-row alert-actions">
    <button class="secondary" disabled={disabled || busy} onclick={() => onopen(row)}>
      {row.subject.kind === 'hearing' ? 'Abrir audiencia' : 'Abrir plazo'}
    </button>
    {#if row.read_at === null}
      {#if uncertain}<button class="secondary" disabled={disabled || busy} onclick={checkRead}
          >Comprobar lectura</button
        >
      {:else}<button class="text-button" disabled={disabled || busy} onclick={markRead}>
          {busy ? 'Guardando lectura...' : 'Marcar como le\u00edda'}
        </button>{/if}
    {/if}
  </div>
</article>
