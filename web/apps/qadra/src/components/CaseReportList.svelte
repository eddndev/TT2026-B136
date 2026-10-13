<script>
  import { activityReport, reportPeriod, phaseLabel } from '../lib/case-reports-presentation.mjs';
  export let rows = [],
    busy = false,
    loaded = false,
    more = false,
    onopen,
    onmore;
</script>

<section class="card report-list" aria-label="Solicitudes y avisos">
  <div class="section-heading">
    <div>
      <h2>Solicitudes y avisos</h2>
      <p>Informes propios conservados por el servidor.</p>
    </div>
  </div>
  {#if busy}<p role="status">Consultando informes...</p>{/if}
  {#each rows as row (row.id)}{@const period = reportPeriod(row)}
    <article class="report-row">
      <div class="report-row-content">
        <span class="report-phase" class:report-ready={row.state === 'ready'}
          >{phaseLabel(row)}</span
        >
        <h3>
          {activityReport(row) ? 'Actividad registrada del' : 'Expedientes creados del'}
          {period.from.slice(0, 10)}
        </h3>
        <p>Hasta {period.before.slice(0, 10)}, sin incluir ese d&#237;a.</p>
        <p class="hint">Solicitud <time datetime={row.requested_at}>{row.requested_at}</time></p>
        {#if row.notice}<span class="report-notice"
            >{row.notice.read_at ? 'Aviso le\u00eddo' : 'Aviso sin leer'}</span
          >{/if}
      </div>
      <button
        class="secondary"
        disabled={busy}
        aria-label={`Consultar informe ${row.id}`}
        onclick={() => onopen(row.id)}>Consultar informe</button
      >
    </article>{/each}
  {#if loaded && !busy && !rows.length}<p class="report-empty">
      Todav&#237;a no hay solicitudes disponibles para tu cuenta.
    </p>{/if}
  {#if more}<div class="report-pagination">
      <p class="hint">Hay m&#225;s solicitudes disponibles.</p>
      <button class="secondary" disabled={busy} onclick={onmore}>Siguientes informes</button>
    </div>{/if}
</section>
