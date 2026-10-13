<script>
  import Icon from '@qadra/ui/Icon.svelte';
  import Pagination from '@qadra/ui/Pagination.svelte';
  import { purposeLabel, dateLabel, publishedLabel } from '../lib/ca-operations-values.mjs';
  import { canRevoke } from '../lib/ca-revocation-model.mjs';

  export let page;
  export let busy = false;
  export let onload;
  export let ondownload;
  export let onrevoke;
  export let mutationBlocked = false;

  const localLabel = { issued: 'Emitido', revoked: 'Revocado', expired: 'Vencido en el registro local' };
  const validityLabel = { valid: 'Dentro de vigencia', expired: 'Vencido', 'not-yet-valid': 'A\u00fan no vigente' };
</script>

<section class="card ca-operation-card" aria-labelledby="certificates-title">
  <div class="ca-list-heading">
    <div><h2 id="certificates-title">Certificados</h2><p>Material p&uacute;blico de la autoridad certificadora.</p></div>
    <button type="button" class="secondary" disabled={busy} onclick={() => onload(page.offset)}>
      <Icon name="search" size={17} />{busy ? 'Consultando...' : 'Actualizar'}
    </button>
  </div>
  <p class="ca-publication-note">El estado local y las fechas del certificado se muestran por separado. La publicaci&oacute;n corresponde a la CRL registrada en Qadra.</p>

  {#if !page.loaded}
    <p class="ca-empty" role="status">{busy ? 'Consultando certificados...' : 'Actualiza para consultar los certificados.'}</p>
  {:else if !page.items.length}
    <p class="ca-empty">No hay certificados en esta consulta.</p>
  {:else}
    <p class="ca-result-count">{page.total} certificados registrados</p>
    <div class="ca-record-list">
      {#each page.items as item (item.serial)}
        <article class="ca-record">
          <div class="ca-record-heading">
            <div><h3>{item.subject}</h3><p>{purposeLabel(item.purpose)} &middot; Serie <code>{item.serial}</code></p></div>
            <span class="badge">{localLabel[item.local_status]}</span>
          </div>
          <dl class="ca-record-facts">
            <div><dt>Vigencia</dt><dd>{validityLabel[item.validity]}<small>{dateLabel(item.not_before)} a {dateLabel(item.not_after)}</small></dd></div>
            <div><dt>Publicaci&oacute;n del estado</dt><dd>{publishedLabel(item.published_status)}</dd></div>
            <div class="ca-fact-wide"><dt>Huella del certificado</dt><dd><code>{item.fingerprint}</code></dd></div>
            <div class="ca-fact-wide"><dt>Huella de la CA</dt><dd><code>{item.root_fingerprint}</code></dd></div>
          </dl>
          <div class="action-row">
            <button type="button" class="secondary" disabled={busy} onclick={() => ondownload(item)}>
              <Icon name="download" size={17} />Descargar certificado
            </button>
            {#if canRevoke(item)}
              <button type="button" class="secondary" disabled={busy || mutationBlocked} onclick={() => onrevoke(item)}>
                Revocar certificado
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
