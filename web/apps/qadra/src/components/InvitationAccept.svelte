<script>
  import Button from './Button.svelte';
  import { onMount, onDestroy } from 'svelte';
  import { createInvitationAcceptanceApi } from '../lib/invitation-acceptance-api.mjs';
  import { invitationOperations } from '../lib/invitation-operation.mjs';
  export let link;
  export let onconsumed;
  export let onclose;
  const api = createInvitationAcceptanceApi();
  let token = link?.status === 'ready' ? link.token : '';
  let mode = token ? 'prepare' : 'invalid';
  let operations,
    enrollment = null,
    receipt = null;
  let password = '',
    code = '',
    error = '',
    saved = false,
    busy = false,
    alive = true;
  onconsumed();
  onMount(async () => {
    if (!token) return;
    let storage;
    try {
      storage = localStorage;
    } catch {
      /* Enrollment also works without local storage. */
    }
    try {
      const value = await invitationOperations(token, storage);
      if (!alive) return;
      operations = value;
      if (value.phase === 'finishing') mode = 'receipt';
    } catch {
      if (alive) {
        mode = 'invalid';
        error = 'No se pudo preparar un registro seguro.';
      }
    }
  });
  onDestroy(() => {
    alive = false;
    api.dispose();
    token = password = code = '';
    enrollment = receipt = null;
  });
  function accept(value) {
    receipt = value;
    operations.clear();
    token = password = code = '';
    enrollment = null;
    mode = 'accepted';
  }
  async function submit(event) {
    event.preventDefault();
    if (busy || !operations || !token) return;
    const size = new TextEncoder().encode(password).length;
    if (size < 12 || size > 1024) {
      error = 'Usa entre 12 y 1024 bytes para tu contrase\u00f1a.';
      return;
    }
    if (mode === 'finish' && (!saved || !/^[0-9]{6}$/.test(code))) return;
    const action = mode;
    busy = true;
    error = '';
    if (action === 'finish') operations.mark('finishing');
    const pending =
      action === 'prepare'
        ? api.prepare(token, operations.prepare, password)
        : action === 'finish'
          ? api.finish(token, operations.finish, password, code)
          : api.receipt(token, operations.finish, password);
    password = code = '';
    try {
      const value = await pending;
      if (!alive) return;
      if (action === 'prepare') {
        enrollment = value;
        operations.mark('prepared');
        saved = false;
        mode = 'finish';
      } else if (action === 'finish' || value.receipt) accept(value.receipt ?? value);
      else {
        operations.mark('prepared');
        mode = enrollment ? 'finish' : 'prepare';
        error = 'No hay un alta confirmada para este intento. Puedes continuar el registro.';
      }
    } catch (failure) {
      if (!alive) return;
      error = failure.message;
      if (action === 'finish') {
        mode =
          failure.code === 'invitation_rejected' || failure.code === 'invitation_limited'
            ? 'finish'
            : 'receipt';
      }
    } finally {
      if (alive) busy = false;
    }
  }
</script>

<h2>{mode === 'finish' ? 'Confirma tu segundo factor' : 'Acepta tu invitaci\u00f3n'}</h2>
{#if error}<div class="notice error" role="alert">{error}</div>{/if}
{#if mode === 'invalid'}
  <p>
    El enlace no es v&#225;lido. Abre el correo original o solicita un reenv&#237;o al
    administrador.
  </p>
{:else if mode === 'accepted'}
  <div class="notice" role="status">
    Cuenta creada: {receipt.email}. Ya puedes iniciar sesi&#243;n con tu contrase&#241;a y segundo
    factor.
  </div>
{:else}
  {#if mode === 'prepare'}
    <p>Elige tu contrase&#241;a. La cuenta se activar&#225; al confirmar tu segundo factor.</p>
    <p>Si ya empezaste este registro, usa la misma contrase&#241;a para recuperarlo.</p>
  {:else if mode === 'receipt'}
    <p>
      Comprueba si tu alta termin&#243; antes de repetirla. Escribe la contrase&#241;a que elegiste.
    </p>
  {:else}
    <p>{enrollment.email}. Agrega esta clave a tu aplicaci&#243;n de autenticaci&#243;n:</p>
    <code class="secret-code">{enrollment.totp_secret_base32}</code>
    <details>
      <summary>Enlace para tu autenticador</summary><code>{enrollment.otpauth_uri}</code>
    </details>
    <p>Conserva estos c&#243;digos de recuperaci&#243;n en un lugar privado antes de continuar:</p>
    <ul>
      {#each enrollment.recovery_codes as recoveryCode}<li><code>{recoveryCode}</code></li>{/each}
    </ul>
    <label class="check-row"
      ><input type="checkbox" bind:checked={saved} disabled={busy} />Ya guard&#233; mis c&#243;digos
      de recuperaci&#243;n</label
    >
  {/if}
  {#if operations && !operations.durable}
    <p class="notice">
      El navegador no permite guardar el identificador de este intento. Conserva esta pesta&#241;a
      abierta hasta confirmar el alta.
    </p>
  {/if}
  <form class="stack" onsubmit={submit}>
    <label
      >Tu contrase&#241;a<input
        type="password"
        required
        autocomplete={mode === 'prepare' ? 'new-password' : 'current-password'}
        bind:value={password}
        disabled={busy}
      /></label
    >
    {#if mode === 'finish'}
      <label
        >C&#243;digo de 6 d&#237;gitos<input
          required
          inputmode="numeric"
          autocomplete="one-time-code"
          pattern={'[0-9]{6}'}
          bind:value={code}
          disabled={busy}
        /></label
      >
    {/if}
    <Button variant="primary" disabled={busy || !operations || (mode === 'finish' && !saved)}
      >{busy
        ? 'Comprobando...'
        : mode === 'prepare'
          ? 'Preparar segundo factor'
          : mode === 'finish'
            ? 'Confirmar alta'
            : 'Comprobar resultado'}</Button
    >
  </form>
{/if}
<Button variant="text" type="button" disabled={busy} onclick={onclose}
  >Ir a iniciar sesi&#243;n</Button
>
