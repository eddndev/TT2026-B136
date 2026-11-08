<script>
  import Button from './Button.svelte';
  // A closing that needs its own confirmation: rejection by the Client,
  // decline by the firm signer, cancellation of a request or abandonment of
  // a contract. None of them is evidence in a package.
  import { contractReason } from '../lib/contract-request-draft.mjs';
  export let outcome,
    disabled = false,
    onconfirm;
  const copy = {
    reject: {
      open: 'Rechazar contrato',
      title: 'Rechazar este contrato',
      text: 'El despacho ver\u00e1 que lo rechazaste. No firmar\u00e1s esta solicitud.',
      confirm: 'Confirmar rechazo',
      required: false,
    },
    decline: {
      open: 'Declinar firma',
      title: 'Declinar la firma del despacho',
      text: 'La solicitud se cerrar\u00e1 sin formalizar el contrato. La firma del Cliente se conserva.',
      confirm: 'Confirmar declinaci\u00f3n',
      required: false,
    },
    cancel: {
      open: 'Cancelar solicitud',
      title: 'Cancelar la solicitud de firma',
      text: 'Las partes ya no podr\u00e1n firmar esta solicitud. Indica el motivo.',
      confirm: 'Confirmar cancelaci\u00f3n',
      required: true,
    },
    abandon: {
      open: 'Abandonar contrato',
      title: 'Abandonar el contrato en borrador',
      text: 'El contrato quedar\u00e1 abandonado y no admitir\u00e1 solicitudes. Indica el motivo.',
      confirm: 'Confirmar abandono',
      required: true,
    },
  };
  $: words = copy[outcome];
  let open = false,
    reason = '',
    error = '',
    busy = false;
  async function confirm() {
    if (busy || disabled) return;
    error = '';
    let value;
    try {
      value = contractReason(reason, words.required);
    } catch (failure) {
      error = failure.message;
      return;
    }
    busy = true;
    try {
      await onconfirm(value);
      open = false;
      reason = '';
    } catch (failure) {
      error = failure.message;
    } finally {
      busy = false;
    }
  }
</script>

{#if !open}<Button variant="secondary" {disabled} onclick={() => (open = true)}>{words.open}</Button
  >
{:else}<section class="notice stack contract-close" aria-label={words.title}>
    <h3>{words.title}</h3>
    <p>{words.text}</p>
    <label
      >{words.required ? 'Motivo' : 'Motivo (opcional)'}
      <textarea maxlength="500" rows="2" bind:value={reason} disabled={busy || disabled}></textarea>
    </label>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    <div class="action-row">
      <Button variant="primary" disabled={busy || disabled} onclick={confirm}
        >{words.confirm}</Button
      >
      <Button variant="secondary" disabled={busy} onclick={() => (open = false)}>Volver</Button>
    </div>
  </section>{/if}

<style>
  textarea {
    width: 100%;
  }
</style>
