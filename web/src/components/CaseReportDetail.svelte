<script>
  import {
    phaseLabel,
    statusLabel,
    scopeLabel,
    failureLabel,
  } from '../lib/case-reports-presentation.mjs';
  export let value = null,
    busy = false,
    downloading = '',
    reading = false,
    onrefresh,
    ondownload,
    onread;
</script>

<section class="card report-detail" aria-label="Detalle de informe" aria-busy={busy}>
  <div class="section-heading">
    <div>
      <h2>Detalle del informe</h2>
      <p>Estado y archivos de la solicitud seleccionada.</p>
    </div>
    <button class="secondary" disabled={busy || downloading || reading} onclick={onrefresh}
      >Actualizar informe</button
    >
  </div>
  {#if busy}<p role="status">Consultando el informe...</p>{/if}
  {#if value && !busy}
    <div class="report-detail-status">
      <span class="report-phase" class:report-ready={value.state === 'ready'}
        >{phaseLabel(value)}</span
      >{#if value.notice}<span class="report-notice"
          >{value.notice.read_at ? 'Aviso le\u00eddo' : 'Aviso sin leer'}</span
        >{/if}
    </div>
    <dl class="report-facts">
      <div>
        <dt>Alcance</dt>
        <dd>{scopeLabel(value.scope)}</dd>
      </div>
      <div>
        <dt>Creaci&#243;n desde (incluida)</dt>
        <dd><time datetime={value.filters.created_from}>{value.filters.created_from}</time></dd>
      </div>
      <div>
        <dt>Creaci&#243;n hasta (excluida)</dt>
        <dd><time datetime={value.filters.created_before}>{value.filters.created_before}</time></dd>
      </div>
      <div>
        <dt>Estado administrativo</dt>
        <dd>{statusLabel(value.filters.status)}</dd>
      </div>
      <div>
        <dt>Filtro de litigante</dt>
        <dd>{value.filters.assigned_litigator || 'Todos los permitidos'}</dd>
      </div>
      <div>
        <dt>Solicitud</dt>
        <dd><time datetime={value.requested_at}>{value.requested_at}</time></dd>
      </div>
      <div>
        <dt>Informe</dt>
        <dd class="report-identifier">{value.id}</dd>
      </div>
    </dl>
    {#if value.state === 'queued' || value.state === 'processing'}<p class="notice">
        El trabajo sigue en el servidor. Actualiza cuando quieras consultar su estado; no necesitas
        mantener esta pantalla abierta.
      </p>{/if}
    {#if value.state === 'retry_waiting'}<p class="notice">
        El servidor conserva la solicitud para un nuevo intento desde la fase {value.phase ===
        'capturing'
          ? 'de captura'
          : 'de generaci\u00f3n'}.{#if value.retry_at}
          Programado a partir de <time datetime={value.retry_at}>{value.retry_at}</time>.{/if}
      </p>{/if}
    {#if value.failure}<p class="notice error">{failureLabel(value.failure)}</p>{/if}
    {#if value.ready}
      <div class="report-capture">
        <h3>Captura compartida por PDF y CSV</h3>
        <p>
          Observada en UTC: <time datetime={value.ready.checked_at}>{value.ready.checked_at}</time>
        </p>
        <p class="hint">Identificador SHA-256 de la captura</p>
        <code>{value.ready.snapshot_digest}</code>
      </div>
      <div class="report-downloads">
        {#each value.ready.artifacts as artifact (artifact.format)}<div class="report-artifact">
            <div>
              <strong>{artifact.format.toUpperCase()}</strong><small
                >{artifact.bytes.toLocaleString('es-MX')} bytes</small
              >
            </div>
            <button
              class="secondary"
              disabled={!!downloading || reading}
              onclick={() => ondownload(artifact.format)}
              >Descargar {artifact.format.toUpperCase()}</button
            >
          </div>{/each}
      </div>
      {#if downloading}<p role="status">
          Validando y descargando {downloading.toUpperCase()}...
        </p>{/if}
      <p class="hint">
        Ambos archivos representan la misma captura. No incorporan cambios posteriores ni sustituyen
        los registros del expediente.
      </p>
    {/if}
    {#if value.notice && !value.notice.read_at}<button
        class="secondary"
        disabled={reading || !!downloading}
        onclick={onread}>{reading ? 'Guardando lectura...' : 'Marcar aviso como le\u00eddo'}</button
      >{/if}
    {#if value.notice?.read_at}<p class="hint">
        Lectura registrada: <time datetime={value.notice.read_at}>{value.notice.read_at}</time>.
      </p>{/if}
  {/if}
</section>
