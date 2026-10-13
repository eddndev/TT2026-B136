<script>
  import { tick } from 'svelte';
  import Icon from '@qadra/ui/Icon.svelte';
  import { purposeLabel, dateLabel } from '../lib/ca-operations-values.mjs';

  export let state;
  export let onedit;
  export let onpreview;
  export let onconfirm;
  export let onresume;
  export let onnew;
  export let ondownload;

  let confirmed = false;
  let previousPreview = '';
  $: previewIdentity = JSON.stringify(state.preview);
  $: if (previousPreview !== previewIdentity) {
    previousPreview = previewIdentity;
    confirmed = false;
  }

  function focus(node) {
    void tick().then(() => { if (node.isConnected) node.focus(); });
  }

  function submit(event) {
    event.preventDefault();
    void onpreview();
  }
</script>

<section class="card ca-operation-card" aria-labelledby="request-title">
  <div class="section-heading">
    <span class="tile-icon"><Icon name="file" size={23} /></span>
    <div><h2 id="request-title">Solicitud de certificado</h2><p>Revisa el material p&uacute;blico antes de emitir.</p></div>
  </div>

  {#if !state.operation_id}
    <form class="stack ca-request-form" onsubmit={submit}>
      <p>El titular conserva su clave privada. Comparte aqu&iacute; &uacute;nicamente su solicitud p&uacute;blica de certificado (CSR).</p>
      <label for="ca-csr">CSR p&uacute;blico en formato PEM
        <textarea id="ca-csr" rows="6" maxlength="16384" required spellcheck="false"
          autocomplete="off" disabled={state.busy} value={state.draft.csr_pem}
          placeholder="-----BEGIN CERTIFICATE REQUEST-----"
          oninput={(event) => onedit({ csr_pem: event.currentTarget.value })}></textarea>
      </label>
      <div class="ca-form-grid">
        <label for="ca-purpose">Prop&oacute;sito
          <select id="ca-purpose" disabled={state.busy} value={state.draft.purpose}
            onchange={(event) => onedit({ purpose: event.currentTarget.value })}>
            <option value="partner">Partner</option>
            <option value="document-signing">Firma documental</option>
          </select>
        </label>
        <label for="ca-holder">Titular revisado
          <input id="ca-holder" required maxlength="500" disabled={state.busy}
            value={state.draft.holder} oninput={(event) => onedit({ holder: event.currentTarget.value })} />
        </label>
        <label for="ca-reference">Referencia de la solicitud
          <input id="ca-reference" required maxlength="500" disabled={state.busy}
            value={state.draft.reference} oninput={(event) => onedit({ reference: event.currentTarget.value })} />
        </label>
        <label for="ca-criterion">Criterio de emisi&oacute;n
          <input id="ca-criterion" required maxlength="500" disabled={state.busy}
            value={state.draft.criterion} oninput={(event) => onedit({ criterion: event.currentTarget.value })} />
        </label>
      </div>
      <div class="action-row">
        <button type="submit" class="secondary" disabled={state.busy}>
          <Icon name="search" size={18} />{state.phase === 'previewing' ? 'Revisando...' : 'Revisar solicitud'}
        </button>
      </div>
    </form>

    {#if state.preview}
      <section class="ca-review" aria-labelledby="review-title">
        <h3 id="review-title" tabindex="-1" use:focus>Confirma la emisi&oacute;n</h3>
        <dl class="settings-list ca-public-facts">
          <div><dt>Sujeto del CSR</dt><dd>{state.preview.subject}</dd></div>
          <div><dt>Prop&oacute;sito</dt><dd>{purposeLabel(state.preview.purpose)}</dd></div>
          <div><dt>Titular revisado</dt><dd>{state.draft.holder}</dd></div>
          <div><dt>Referencia</dt><dd>{state.draft.reference}</dd></div>
          <div><dt>Criterio</dt><dd>{state.draft.criterion}</dd></div>
          <div><dt>Huella de la CA</dt><dd><code>{state.preview.root_fingerprint}</code></dd></div>
          <div><dt>Huella del CSR</dt><dd><code>{state.preview.csr_sha256}</code></dd></div>
          <div><dt>Huella de la clave p&uacute;blica</dt><dd><code>{state.preview.spki_sha256}</code></dd></div>
        </dl>
        <label class="checkbox ca-confirmation">
          <input type="checkbox" bind:checked={confirmed} disabled={state.busy} />
          He revisado el titular, el prop&oacute;sito y la CA que emitir&aacute; este certificado.
        </label>
        <button type="button" class="primary" disabled={state.busy || !confirmed}
          onclick={() => { if (confirmed) void onconfirm(); }}>
          <Icon name="shield" size={18} />Confirmar y emitir
        </button>
      </section>
    {/if}
  {:else if state.phase === 'completed' && state.receipt}
    <div class="notice success" role="status"><Icon name="check" size={18} /> Emisi&oacute;n confirmada.</div>
    <dl class="settings-list ca-public-facts">
      <div><dt>Titular</dt><dd>{state.receipt.holder}</dd></div>
      <div><dt>Prop&oacute;sito</dt><dd>{purposeLabel(state.receipt.purpose)}</dd></div>
      <div><dt>Serie</dt><dd><code>{state.receipt.serial}</code></dd></div>
      <div><dt>Huella del certificado</dt><dd><code>{state.receipt.fingerprint}</code></dd></div>
      <div><dt>Referencia</dt><dd>{state.receipt.reference}</dd></div>
      <div><dt>Criterio</dt><dd>{state.receipt.criterion}</dd></div>
      <div><dt>Registro de la solicitud</dt><dd>{dateLabel(state.receipt.recorded_at)}</dd></div>
      <div><dt>Operaci&oacute;n</dt><dd><code>{state.receipt.operation_id}</code></dd></div>
    </dl>
    <div class="action-row ca-card-actions">
      <button type="button" class="primary" disabled={state.busy} onclick={() => ondownload(state.receipt)}>
        <Icon name="download" size={18} />Descargar certificado
      </button>
      <button type="button" class="secondary" disabled={state.busy} onclick={onnew}>Nueva solicitud</button>
    </div>
  {:else}
    <section class="ca-recovery" aria-labelledby="recovery-title">
      <h3 id="recovery-title" tabindex="-1" use:focus>
        {state.phase === 'issuing' ? 'Procesando la solicitud' : 'Resultado por confirmar'}
      </h3>
      <p>La solicitud conserva este identificador. Retomarla consulta o completa la misma emisi&oacute;n.</p>
      <code class="ca-operation-id">{state.operation_id}</code>
      {#if state.preview}<p>Titular: {state.draft.holder} &middot; {purposeLabel(state.draft.purpose)}</p>{/if}
      <p>Si vuelves a iniciar sesi&oacute;n, busca la operaci&oacute;n pendiente en Historial.</p>
      <button type="button" class="primary" disabled={state.busy} onclick={() => onresume(state.operation_id)}>
        <Icon name="clock" size={18} />{state.busy ? 'Procesando...' : 'Retomar esta operaci\u00f3n'}
      </button>
    </section>
  {/if}
</section>
