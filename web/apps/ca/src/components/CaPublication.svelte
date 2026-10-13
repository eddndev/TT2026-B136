<script>
  import Icon from '@qadra/ui/Icon.svelte';
  import { dateLabel } from '../lib/ca-operations-values.mjs';

  export let state;
  export let busy = false;
  export let blocked = false;
  export let onrefresh;
  export let onconfirm;
  export let onresume;
  export let onreset;
  export let onhistory;

  let acknowledged = false;
  let previousStatus = '';
  $: statusIdentity = JSON.stringify(state.status);
  $: if (previousStatus !== statusIdentity) {
    previousStatus = statusIdentity;
    acknowledged = false;
  }
</script>

<section class="card ca-operation-card" aria-labelledby="publication-title">
  <div class="ca-list-heading">
    <div><h2 id="publication-title">Publicaci&oacute;n de la CRL</h2>
      <p>Consulta la lista de revocaci&oacute;n registrada en Qadra.</p></div>
    {#if !state.operation_id}
      <button type="button" class="secondary" disabled={busy || state.busy} onclick={onrefresh}>
        <Icon name="search" size={17} />Actualizar estado
      </button>
    {/if}
  </div>
  {#if state.error}<div class="notice error" role="alert">{state.error}</div>{/if}
  {#if blocked}<p class="notice">Retoma primero la emisi&oacute;n o revocaci&oacute;n pendiente antes de publicar.</p>{/if}

  {#if state.phase === 'completed' && state.receipt}
    <div class="notice success" role="status">CRL publicada en Qadra.</div>
    <dl class="settings-list ca-public-facts">
      <div><dt>Revisi&oacute;n publicada</dt><dd>{state.receipt.revision}</dd></div>
      <div><dt>Revisi&oacute;n anterior</dt><dd>{state.receipt.expected_revision}</dd></div>
      <div><dt>N&uacute;mero de CRL</dt><dd>{state.receipt.crl_number}</dd></div>
      <div><dt>Huella de la CA</dt><dd><code>{state.receipt.root_fingerprint}</code></dd></div>
      <div><dt>Registro de la operaci&oacute;n</dt><dd>{dateLabel(state.receipt.recorded_at)}</dd></div>
      <div><dt>Operaci&oacute;n</dt><dd><code>{state.receipt.operation_id}</code></dd></div>
    </dl>
    <button type="button" class="secondary ca-card-actions" disabled={busy} onclick={onreset}>Volver al estado de la CRL</button>
  {:else if state.operation_id}
    <div class="ca-recovery ca-card-actions">
      <h3>{state.phase === 'publishing' ? 'Publicaci\u00f3n en curso' : 'Publicaci\u00f3n por confirmar'}</h3>
      <p>Retomar conserva la misma operaci&oacute;n y la revisi&oacute;n original. No inicies otra renovaci&oacute;n.</p>
      <dl class="settings-list ca-public-facts">
        <div><dt>Operaci&oacute;n</dt><dd><code>{state.operation_id}</code></dd></div>
        <div><dt>Revisi&oacute;n original</dt><dd>{state.expected_revision}</dd></div>
      </dl>
      <p>El mantenimiento puede continuar activo. Si vuelves a iniciar sesi&oacute;n, retoma la publicaci&oacute;n desde Historial.</p>
      <button type="button" class="primary" disabled={busy || state.busy || blocked} onclick={onresume}>
        <Icon name="clock" size={18} />{state.busy ? 'Procesando...' : 'Retomar esta publicaci\u00f3n'}
      </button>
    </div>
  {:else if state.status}
    <dl class="settings-list ca-public-facts ca-card-actions">
      <div><dt>Revisi&oacute;n actual</dt><dd>{state.status.revision}</dd></div>
      <div><dt>N&uacute;mero de CRL</dt><dd>{state.status.crl_number}</dd></div>
      <div><dt>Vigencia observada</dt><dd>{state.status.valid_now ? 'Vigente' : 'Fuera de vigencia'}<br />
        <small>{dateLabel(state.status.valid_from)} a {dateLabel(state.status.valid_until)}</small></dd></div>
      <div><dt>Consulta realizada</dt><dd>{dateLabel(state.status.checked_at)}</dd></div>
      <div><dt>Certificados en la CRL</dt><dd>{state.status.revoked_count}</dd></div>
      <div><dt>Huella de la CA</dt><dd><code>{state.status.root_fingerprint}</code></dd></div>
      <div><dt>Huella de la CRL</dt><dd><code>{state.status.crl_digest}</code></dd></div>
    </dl>
    {#if state.status.maintenance_pending}
      <p class="notice">Hay mantenimiento pendiente. Revisa el historial y retoma su operaci&oacute;n.</p>
      <button type="button" class="secondary ca-card-actions" disabled={busy} onclick={onhistory}>Ver historial</button>
    {:else}
      <p class="ca-publication-note">Renovar y publicar la CRL requiere una ventana de mantenimiento. El acceso a Qadra se interrumpe durante esta operaci&oacute;n.</p>
      <label class="checkbox ca-confirmation">
        <input type="checkbox" bind:checked={acknowledged} disabled={busy || state.busy || blocked} />
        Confirmo la revisi&oacute;n {state.status.revision} y autorizo la ventana de mantenimiento para publicar la nueva CRL.
      </label>
      <button type="button" class="primary" disabled={busy || state.busy || blocked || !acknowledged}
        onclick={() => { if (acknowledged) void onconfirm({ expected_revision: state.status.revision, acknowledge_maintenance: true }); }}>
        <Icon name="upload" size={18} />Renovar y publicar CRL
      </button>
    {/if}
  {:else}
    <p class="ca-empty" role="status">{state.busy ? 'Consultando la CRL...' : 'Consulta el estado actual antes de publicar.'}</p>
  {/if}
</section>
