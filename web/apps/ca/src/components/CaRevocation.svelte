<script>
  import { tick } from 'svelte';
  import Icon from '@qadra/ui/Icon.svelte';
  import { purposeLabel } from '../lib/ca-operations-values.mjs';

  export let state;
  export let busy = false;
  export let blocked = false;
  export let onreason;
  export let onconfirm;
  export let onresume;
  export let oncancel;

  let confirmed = false;
  let previousReview = '';
  $: reviewIdentity = JSON.stringify([state.certificate?.serial, state.certificate?.fingerprint, state.reason]);
  $: if (previousReview !== reviewIdentity) {
    previousReview = reviewIdentity;
    confirmed = false;
  }

  function focus(node) {
    void tick().then(() => { if (node.isConnected) node.focus(); });
  }
</script>

{#if state.certificate}
  <section class="card ca-operation-card ca-revocation" aria-labelledby="revocation-title">
    <div class="section-heading">
      <span class="tile-icon"><Icon name="shield" size={23} /></span>
      <div><h2 id="revocation-title" tabindex="-1" use:focus>Revocaci&oacute;n de certificado</h2>
        <p>{state.certificate.subject || 'Certificado del historial'}
          {#if state.certificate.purpose} &middot; {purposeLabel(state.certificate.purpose)}{/if}</p></div>
    </div>
    {#if state.error}<div class="notice error" role="alert">{state.error}</div>{/if}
    {#if blocked}<p class="notice">Retoma primero la emisi&oacute;n pendiente antes de revocar otro certificado.</p>{/if}

    <dl class="settings-list ca-public-facts">
      <div><dt>Serie que se revocar&aacute;</dt><dd><code>{state.certificate.serial}</code></dd></div>
      <div><dt>Huella del certificado</dt><dd><code>{state.certificate.fingerprint}</code></dd></div>
      {#if state.certificate.root_fingerprint}
        <div><dt>Huella de la CA</dt><dd><code>{state.certificate.root_fingerprint}</code></dd></div>
      {/if}
      {#if state.operation_id}
        <div><dt>Operaci&oacute;n</dt><dd><code>{state.operation_id}</code></dd></div>
        <div><dt>Motivo original</dt><dd>{state.reason}</dd></div>
      {/if}
    </dl>

    {#if state.phase === 'review'}
      <p class="ca-publication-note">Esta acci&oacute;n registra la revocaci&oacute;n local del certificado exacto. Su publicaci&oacute;n en Qadra se realiza por separado.</p>
      <label class="ca-revocation-reason" for="ca-revocation-reason">Motivo de la revocaci&oacute;n
        <input id="ca-revocation-reason" required maxlength="500" value={state.reason}
          disabled={busy || blocked} oninput={(event) => onreason(event.currentTarget.value)} />
      </label>
      <label class="checkbox ca-confirmation">
        <input type="checkbox" bind:checked={confirmed} disabled={busy || blocked} />
        Confirmo la serie, la huella y el motivo de esta revocaci&oacute;n.
      </label>
      <div class="action-row">
        <button type="button" class="primary" disabled={busy || blocked || !confirmed || !state.reason.trim()}
          onclick={() => { if (confirmed) void onconfirm(state.certificate); }}>
          Confirmar revocaci&oacute;n local
        </button>
        <button type="button" class="secondary" disabled={busy} onclick={oncancel}>Cancelar</button>
      </div>
    {:else if state.phase === 'completed'}
      <div class="notice success" role="status">Revocaci&oacute;n registrada localmente.</div>
      <p class="ca-publication-note">Pendiente de publicaci&oacute;n en Qadra. Este recibo confirma el cambio local del certificado.</p>
      <button type="button" class="secondary ca-card-actions" disabled={busy} onclick={oncancel}>Cerrar recibo</button>
    {:else}
      <p class="ca-publication-note" role="status">
        {state.phase === 'submitting' ? 'Procesando la revocaci\u00f3n...' : 'Resultado por confirmar. Retomar conserva la serie, la huella y el motivo originales.'}
      </p>
      <button type="button" class="primary ca-card-actions" disabled={busy || state.busy || blocked} onclick={onresume}>
        <Icon name="clock" size={18} />{state.busy ? 'Procesando...' : 'Retomar esta revocaci\u00f3n'}
      </button>
    {/if}
  </section>
{/if}
