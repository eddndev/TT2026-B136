<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import {
    withdrawalIntent,
    reconcileOwnerIntent,
    uncertainOwnerWrite,
  } from '../lib/owner-certificate-intent.mjs';
  import {
    createOwnerBindingDraft,
    freshOwnerBinding,
    denyOwnerDrafts,
    ownerFailure,
  } from '../lib/owner-binding-draft.mjs';
  export let api,
    scoped,
    principalId,
    bindingId,
    original = null,
    saved = null;
  export let onconfirmed, oncancel, ondenied;
  const session = getContext('session-drafts');
  let dialog,
    command = null,
    mode = 'editing',
    ready = false,
    busy = true;
  let alive = true,
    completed = false,
    generation = 0,
    error = '';
  const recovery = createOwnerBindingDraft({
    session,
    principalId,
    bindingId,
    action: 'withdraw',
    capture,
  });
  const admitted = () => alive && !completed && recovery.admitted();
  const active = (request) => request === generation && admitted();
  function capture() {
    return {
      original,
      command,
      mode: mode === 'unconfirmed' ? 'uncertain' : mode,
      prepared: null,
      signatureBase64: '',
      signatureName: '',
    };
  }
  function denied(failure) {
    if (failure.status !== 403) return;
    denyOwnerDrafts(session);
    completed = true;
    dialog.close();
    ondenied(failure);
  }
  async function load() {
    const request = ++generation;
    busy = true;
    error = '';
    const fresh = () => freshOwnerBinding(api, scoped, principalId, () => active(request));
    try {
      if (saved) {
        const outcome = await recovery.restore(saved, fresh, (value, context) => {
          original = value.original;
          command = value.command;
          mode = value.mode;
          if (
            mode === 'editing' &&
            reconcileOwnerIntent(command, context.current) !== 'unconfirmed'
          )
            mode = 'uncertain';
        });
        if (!active(request)) return;
        if (outcome.status !== 'restored') throw new Error('No se pudo recuperar el retiro.');
      } else {
        const context = await fresh();
        if (!context || !active(request)) return;
        command = withdrawalIntent(original);
        if (reconcileOwnerIntent(command, context.current) !== 'unconfirmed') mode = 'uncertain';
        recovery.register();
      }
      ready = true;
    } catch (failure) {
      if (active(request)) {
        error = ownerFailure(failure);
        denied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function confirmed(value) {
    if (reconcileOwnerIntent(command, value) !== 'matched')
      throw new Error('El recibo no corresponde a la evidencia del retiro.');
    recovery.close();
    completed = true;
    generation++;
    dialog.close();
    onconfirmed(value);
  }
  async function submit() {
    if (!ready || busy || !admitted() || !['editing', 'unconfirmed'].includes(mode)) return;
    command ??= withdrawalIntent(original);
    const request = ++generation;
    busy = true;
    mode = 'uncertain';
    error = '';
    try {
      const value = await scoped.withdraw(bindingId, command.data.expected_revision);
      if (active(request)) confirmed(value);
    } catch (failure) {
      if (active(request)) {
        mode = uncertainOwnerWrite(failure) ? 'uncertain' : 'rejected';
        error = ownerFailure(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function check() {
    if (!ready || busy || !admitted() || !command) return;
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const value = await scoped.get(bindingId);
      if (!active(request)) return;
      const result = reconcileOwnerIntent(command, value);
      if (result === 'matched') {
        confirmed(value);
        return;
      }
      mode = result === 'unconfirmed' ? 'unconfirmed' : 'conflict';
      error =
        result === 'unconfirmed'
          ? 'El recibo conserva este vinculo sin retirar. Puedes confirmar de nuevo el mismo retiro.'
          : 'El recibo no coincide con la evidencia original. El retiro sigue bloqueado.';
    } catch (failure) {
      if (active(request)) {
        error = ownerFailure(failure);
        denied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function close(event) {
    event?.preventDefault();
    if (busy || !admitted()) return;
    recovery.close();
    completed = true;
    generation++;
    dialog.close();
    oncancel();
  }
  onMount(() => {
    dialog.showModal();
    load();
  });
  onDestroy(() => {
    alive = false;
    generation++;
    recovery.dispose();
  });
</script>

<dialog
  class="upload-dialog"
  bind:this={dialog}
  aria-label="Retirar mi certificado"
  oncancel={close}
>
  <div class="dialog-heading">
    <div>
      <span class="eyebrow">MI CUENTA</span>
      <h2>Retirar mi certificado</h2>
    </div>
  </div>
  <p class="identifier">{bindingId}</p>
  <p>
    El retiro es terminal para este v&#237;nculo. Conserva su evidencia hist&#243;rica; no revoca el
    certificado. Las sesiones abiertas con este v&#237;nculo quedan inv&#225;lidas; el acceso por
    contrase&#241;a y MFA es independiente.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if !ready}<p role="status">Comprobando acceso y estado de la cuenta...</p>
    {#if !busy}<button class="secondary" onclick={load}>Volver a consultar</button>{/if}
  {:else if mode !== 'editing'}
    <p class="notice">
      Comprueba el recibo de este UUID. La consulta del v&#237;nculo actual no confirma el retiro.
      Cancelar descarta este intento; no revierte un retiro confirmado en el servidor.
    </p>
    <button class="secondary" disabled={busy} onclick={check}>Comprobar retiro</button>
  {/if}
  <div class="dialog-actions">
    <button class="secondary" disabled={busy} onclick={close}>Cancelar retiro</button>
    <button
      class="primary"
      disabled={!ready || busy || !['editing', 'unconfirmed'].includes(mode)}
      onclick={submit}>Confirmar retiro</button
    >
  </div>
</dialog>

<style>
  .identifier {
    overflow-wrap: anywhere;
  }
</style>
