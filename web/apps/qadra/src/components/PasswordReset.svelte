<script>
  import { onDestroy } from 'svelte';
  import { createPasswordResetApi } from '../lib/password-reset-api.mjs';
  export let link = null;
  export let onclose;
  export let onconsumed;
  const api = createPasswordResetApi();
  let token = link?.status === 'ready' ? link.token : '';
  let mode = link ? (token ? 'complete' : 'invalid') : 'request';
  let email = '',
    password = '',
    confirmation = '',
    error = '',
    busy = false;
  let alive = true;
  // The parent releases its token reference as soon as this form takes ownership.
  onconsumed();
  onDestroy(() => {
    alive = false;
    token = password = confirmation = email = '';
  });
  async function request(event) {
    event.preventDefault();
    if (busy) return;
    busy = true;
    error = '';
    const result = await api.request(email.trim());
    if (!alive) return;
    email = '';
    busy = false;
    if (result === 'accepted') mode = 'accepted';
    else error = 'La recuperaci\u00f3n no est\u00e1 disponible. Int\u00e9ntalo m\u00e1s tarde.';
  }
  async function complete(event) {
    event.preventDefault();
    if (busy || !token) return;
    error = '';
    if (password !== confirmation) {
      error = 'Las contrase\u00f1as no coinciden.';
      return;
    }
    const size = new TextEncoder().encode(password).length;
    if (size < 12 || size > 1024) {
      error = 'Usa entre 12 y 1024 bytes para la nueva contrase\u00f1a.';
      return;
    }
    busy = true;
    const pending = api.complete(token, password);
    token = password = confirmation = '';
    const result = await pending;
    if (!alive) return;
    busy = false;
    mode = result;
  }
</script>

<span class="eyebrow">RECUPERAR EL ACCESO</span>
<h2>{mode === 'complete' ? 'Elige una nueva contrase\u00f1a.' : 'Recupera tu acceso.'}</h2>
{#if mode === 'request'}
  <p>Solicita un enlace privado para cambiar tu contrase&ntilde;a.</p>
  {#if error}<div class="notice error" role="alert">{error}</div>{/if}
  <form class="stack" onsubmit={request}>
    <label
      >Correo electr&oacute;nico<input
        type="email"
        autocomplete="username"
        required
        bind:value={email}
        disabled={busy}
      /></label
    >
    <button class="primary" disabled={busy}>{busy ? 'Solicitando...' : 'Solicitar enlace'}</button>
  </form>
{:else if mode === 'complete'}
  <p>El cambio conserva tu segundo factor de autenticaci&oacute;n.</p>
  {#if error}<div class="notice error" role="alert">{error}</div>{/if}
  <form class="stack" onsubmit={complete}>
    <label
      >Nueva contrase&ntilde;a<input
        type="password"
        autocomplete="new-password"
        required
        bind:value={password}
        disabled={busy}
      /></label
    >
    <label
      >Repite la nueva contrase&ntilde;a<input
        type="password"
        autocomplete="new-password"
        required
        bind:value={confirmation}
        disabled={busy}
      /></label
    >
    <small>Usa entre 12 y 1024 bytes. No reutilices tu contrase&ntilde;a anterior.</small>
    <button class="primary" disabled={busy}
      >{busy ? 'Cambiando...' : 'Cambiar contrase\u00f1a'}</button
    >
  </form>
{:else if mode === 'accepted'}
  <div class="notice" role="status">
    Si la solicitud puede atenderse, podr&aacute;s recibir un enlace. Esta respuesta no confirma un
    env&iacute;o.
  </div>
{:else if mode === 'changed'}
  <div class="notice" role="status">
    Contrase&ntilde;a actualizada. Inicia sesi&oacute;n con la nueva contrase&ntilde;a.
  </div>
  <p>Conserva tu segundo factor</p>
{:else if mode === 'uncertain'}
  <div class="notice error" role="alert">
    No pudimos confirmar el cambio. Prueba iniciar sesi&oacute;n con tu nueva contrase&ntilde;a o
    solicita otro enlace. No se reenviar&aacute; este cambio.
  </div>
{:else if mode === 'rejected'}
  <div class="notice error" role="alert">
    El enlace ya no puede usarse. Solicita uno nuevo desde el inicio de sesi&oacute;n.
  </div>
{:else}
  <div class="notice error" role="alert">
    El enlace no es v&aacute;lido o no se pudo abrir de forma segura. Solicita otro enlace.
  </div>
{/if}
<button class="text-button" type="button" disabled={busy} onclick={onclose}
  >Volver al inicio de sesi&oacute;n</button
>
