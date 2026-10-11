<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import { invitationOwnerDraft } from '../lib/invitation-owner-draft.mjs';
  export let api, user;
  const client = api.invitations();
  let email = '',
    role = 'paralegal',
    items = [],
    next = null;
  let intent = null,
    pending = false,
    confirming = null,
    busy = false;
  let alive = true,
    error = '',
    notice = '';
  const recovery = invitationOwnerDraft(getContext('session-drafts'), api, user.id, () => ({
    intent,
  }));
  const admitted = () => alive && recovery.admitted();
  const statuses = {
    pending: 'Pendiente de aceptar',
    pending_mfa: 'Pendiente de confirmar MFA',
    accepted: 'Cuenta creada',
    expired: 'Enlace vencido o retirado',
  };
  const delivery = {
    pending: 'Correo pendiente',
    sent: 'Correo aceptado por el proveedor',
    uncertain: 'Envio por comprobar',
    rejected: 'Correo rechazado',
  };
  async function refresh(more = false) {
    if (!admitted() || busy) return;
    busy = true;
    error = '';
    try {
      const page = await client.list(more ? next : null);
      if (!admitted()) return;
      items = more
        ? [...new Map([...items, ...page.items].map((row) => [row.id, row])).values()].sort(
            (a, b) => a.id.localeCompare(b.id),
          )
        : page.items;
      next = page.next;
    } catch (failure) {
      if (admitted()) error = failure.message;
    } finally {
      if (alive) busy = false;
    }
  }
  function remember(value) {
    intent = value;
    recovery.register();
    notice = '';
    error = '';
  }
  function confirmed(row) {
    recovery.close();
    intent = null;
    confirming = null;
    pending = false;
    email = '';
    notice = `Invitacion registrada para ${row.email}.`;
    items = [row, ...items.filter((item) => item.id !== row.id)].sort((a, b) =>
      a.id.localeCompare(b.id),
    );
  }
  async function send() {
    if (!admitted() || busy || !intent) return;
    busy = true;
    error = '';
    try {
      const row =
        intent.kind === 'issue'
          ? await client.issue(intent.data)
          : await client.resend(intent.id, intent.data);
      if (admitted()) confirmed(row);
    } catch {
      if (admitted())
        error = 'No se confirmo la invitacion. Comprueba el intento antes de continuar.';
    } finally {
      if (alive) busy = false;
    }
  }
  function issue(event) {
    event.preventDefault();
    if (!admitted() || busy || intent || pending) return;
    const operation_id = crypto.randomUUID();
    remember({
      kind: 'issue',
      id: operation_id,
      data: { operation_id, email: email.trim().toLowerCase(), role },
    });
    send();
  }
  function resend() {
    if (!confirming || !admitted() || busy || intent || pending) return;
    remember({
      kind: 'resend',
      id: confirming.id,
      data: { operation_id: crypto.randomUUID(), expected_revision: confirming.revision },
    });
    confirming = null;
    send();
  }
  async function consult() {
    if (!admitted() || busy || !intent) return;
    busy = true;
    error = '';
    try {
      const row = await client.get(intent.id);
      if (!admitted()) return;
      if (row.operation_id === intent.data.operation_id) confirmed(row);
      else
        error =
          'La invitacion tiene otro envio vigente. Reintentar el mismo intento permite consultar su recibo original.';
    } catch {
      if (admitted())
        error =
          'No se pudo confirmar este intento. Puedes reintentar el mismo envio o consultar mas tarde.';
    } finally {
      if (alive) busy = false;
    }
  }
  async function restore() {
    if (!admitted() || busy) return;
    busy = true;
    error = '';
    try {
      const result = await recovery.restore((value) => {
        intent = value;
      });
      if (admitted() && result.status === 'restored') pending = false;
    } catch {
      if (admitted()) error = 'No se pudo recuperar la invitacion con los permisos actuales.';
    } finally {
      if (alive) busy = false;
    }
  }
  function discard() {
    if (!admitted() || busy) return;
    recovery.close();
    intent = null;
    pending = false;
    error = '';
  }
  onMount(() => {
    pending = recovery.pending();
    refresh();
  });
  onDestroy(() => {
    recovery.dispose();
    alive = false;
    client.dispose();
    intent = confirming = null;
    items = [];
  });
</script>

<section class="card admin-panel" aria-label="Invitaciones al despacho" aria-busy={busy}>
  <h2>Invitaciones al despacho</h2>
  <p>
    El integrante elegira su contrasena y confirmara su segundo factor desde el enlace privado del
    correo.
  </p>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if notice}<p class="notice" role="status">{notice}</p>{/if}
  {#if pending}
    <p>Hay una invitacion por comprobar de esta cuenta.</p>
    <button class="secondary" disabled={busy} onclick={restore}>Retomar invitacion</button>
  {:else if intent}
    <p>Se conserva el identificador de este intento para comprobarlo sin crear otro.</p>
    <button class="secondary" disabled={busy} onclick={consult}>Comprobar invitacion</button>
    <button class="secondary" disabled={busy} onclick={send}>Reintentar el mismo envio</button>
    <button class="text-button" disabled={busy} onclick={discard}>Cerrar intento</button>
  {:else if confirming}
    <p>
      El enlace anterior dejara de funcionar, junto con su preparacion MFA. Se enviara un nuevo
      enlace a {confirming.email}.
    </p>
    <button class="primary" disabled={busy} onclick={resend}>Confirmar reenvio</button>
    <button
      class="text-button"
      disabled={busy}
      onclick={() => {
        confirming = null;
      }}>Cancelar reenvio</button
    >
  {:else}
    <form class="stack" onsubmit={issue}>
      <label
        >Correo de la invitacion<input
          type="email"
          required
          maxlength="254"
          autocomplete="off"
          bind:value={email}
          disabled={busy}
        /></label
      >
      <label
        >Rol de la invitacion<select bind:value={role} disabled={busy}
          ><option value="paralegal">Asistente</option><option value="litigator">Litigante</option
          ></select
        ></label
      >
      <button class="primary" disabled={busy}>Enviar invitacion</button>
    </form>
  {/if}
  <button class="secondary" disabled={busy} onclick={() => refresh()}
    >Actualizar invitaciones</button
  >
  <ul>
    {#each items as row (row.id)}
      <li>
        <strong>{row.email}</strong> - {row.role === 'litigator' ? 'Litigante' : 'Asistente'}<br />
        {statuses[row.status]}. {delivery[row.delivery]}.
        <p>Vence: {new Date(row.expires_at).toLocaleString()}.</p>
        {#if row.status !== 'accepted'}<button
            class="text-button"
            disabled={busy || !!intent || pending}
            onclick={() => {
              confirming = row;
              notice = '';
            }}>Reenviar a {row.email}</button
          >{/if}
      </li>
    {/each}
  </ul>
  {#if next}<button class="secondary" disabled={busy} onclick={() => refresh(true)}
      >Cargar mas invitaciones</button
    >{/if}
</section>
