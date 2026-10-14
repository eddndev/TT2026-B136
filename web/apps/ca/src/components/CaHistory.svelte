<script>
  import Icon from '@qadra/ui/Icon.svelte';
  import Pagination from '@qadra/ui/Pagination.svelte';
  import { dateLabel } from '../lib/ca-operations-values.mjs';

  export let page;
  export let busy = false;
  export let blockedOperation = null;
  export let onload;
  export let onresume;
  export let ondownload;
  const kindLabel = { issue: 'Emisi\u00f3n', revoke: 'Revocaci\u00f3n', publication: 'Publicaci\u00f3n de CRL' };
  const resumeLabel = { issue: 'Retomar emisi\u00f3n', revoke: 'Retomar revocaci\u00f3n', publication: 'Retomar publicaci\u00f3n' };
</script>

<section class="card ca-operation-card" aria-labelledby="history-title">
  <div class="ca-list-heading">
    <div><h2 id="history-title">Historial de operaciones</h2><p>Emisiones, revocaciones y publicaciones registradas.</p></div>
    <button type="button" class="secondary" disabled={busy} onclick={() => onload(page.offset)}>
      <Icon name="search" size={17} />{busy ? 'Consultando...' : 'Actualizar'}
    </button>
  </div>
  <p class="ca-publication-note">Retomar una operaci&oacute;n pendiente consulta o completa el mismo registro, con sus datos originales.</p>

  {#if !page.loaded}
    <p class="ca-empty" role="status">{busy ? 'Consultando historial...' : 'Actualiza para consultar las operaciones.'}</p>
  {:else if !page.items.length}
    <p class="ca-empty">No hay operaciones registradas en esta consulta.</p>
  {:else}
    <p class="ca-result-count">{page.total} operaciones registradas</p>
    <div class="ca-record-list">
      {#each page.items as item (item.operation_id)}
        <article class="ca-record">
          <div class="ca-record-heading">
            <div><h3>{kindLabel[item.kind]}{#if item.holder}: {item.holder}{/if}</h3><p>{dateLabel(item.recorded_at)}</p></div>
            <span class="badge" class:success={item.state === 'completed'}>
              {item.state === 'completed' ? 'Completada' : item.state === 'rejected' ? 'Rechazada sin emitir' : 'Pendiente'}
            </span>
          </div>
          <dl class="ca-record-facts">
            {#if item.kind === 'issue'}
              <div><dt>Referencia</dt><dd>{item.reference}</dd></div>
              <div><dt>Criterio de emisi&oacute;n</dt><dd>{item.criterion}</dd></div>
              {#if item.state === 'rejected'}<div><dt>Motivo</dt><dd>El sujeto del CSR no cumple la pol&iacute;tica de la CA. No se emiti&oacute; un certificado.</dd></div>{/if}
            {:else if item.kind === 'revoke'}
              <div class="ca-fact-wide"><dt>Motivo de revocaci&oacute;n</dt><dd>{item.reason}</dd></div>
            {:else}
              <div><dt>Revisi&oacute;n original</dt><dd>{item.expected_revision}</dd></div>
              {#if item.revision}<div><dt>Revisi&oacute;n publicada</dt><dd>{item.revision}</dd></div>{/if}
            {/if}
            <div><dt>Operaci&oacute;n</dt><dd><code>{item.operation_id}</code></dd></div>
            <div><dt>Operador que registr&oacute; la solicitud</dt><dd><code>{item.operator_id}</code></dd></div>
            {#if item.serial}
              <div><dt>Serie</dt><dd><code>{item.serial}</code></dd></div>
              <div class="ca-fact-wide"><dt>Huella del certificado</dt><dd><code>{item.fingerprint}</code></dd></div>
            {/if}
          </dl>
          <div class="action-row">
            {#if item.state === 'pending'}
              <button type="button" class="primary"
                disabled={busy || Boolean(blockedOperation && blockedOperation !== item.operation_id)}
                onclick={() => onresume(item)}>
                <Icon name="clock" size={17} />{resumeLabel[item.kind]}
              </button>
            {:else if item.kind !== 'publication' && item.serial}
              <button type="button" class="secondary" disabled={busy} onclick={() => ondownload(item)}>
                <Icon name="download" size={17} />Descargar certificado
              </button>
            {/if}
          </div>
        </article>
      {/each}
    </div>
  {/if}

  <Pagination offset={page.offset} limit={page.limit} count={page.loaded ? page.items.length : 0}
    hasMore={page.loaded && page.offset + page.items.length < page.total} {busy} onchange={onload} />
</section>
