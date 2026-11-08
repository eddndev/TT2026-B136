<script>
  import Button from './Button.svelte';
  import { getContext, onMount, onDestroy } from 'svelte';
  import Enrollment from './Enrollment.svelte';
  import Icon from '@qadra/ui/Icon.svelte';
  import MemberEnrollmentFields from './MemberEnrollmentFields.svelte';
  import {
    authorizeEnrollment,
    createMemberEnrollmentDraft,
    findEnrollmentAccount,
  } from '../lib/member-enrollment-draft.mjs';
  import { accountName } from '../lib/account-name.mjs';
  export let api, user, oncomplete, onreturn;
  const session = getContext('session-drafts'),
    principalId = user.id;
  const members = api.members();
  let name = '',
    email = '',
    role = 'paralegal',
    password = '',
    enrollment = null;
  let uncertain = false,
    pending = false,
    ready = false,
    busy = false,
    writing = false;
  let alive = true,
    generation = 0,
    error = '',
    observed = null;
  const recovery = createMemberEnrollmentDraft({
    session,
    principalId,
    capture: () => ({ name, email, role, uncertain }),
  });
  const admitted = () => alive && recovery.admitted();
  const active = (request) => request === generation && admitted();
  function reset() {
    name = '';
    email = '';
    role = 'paralegal';
    password = '';
    enrollment = null;
    uncertain = false;
    observed = null;
    pending = false;
  }
  function fail(failure) {
    if (failure.status === 403) {
      recovery.deny();
      reset();
      ready = false;
      error = 'Tu cuenta ya no tiene permiso para crear integrantes.';
    } else
      error = 'No se pudo consultar el acceso o el directorio. Vuelve a consultar explicitamente.';
  }
  async function authorize() {
    if (busy || !admitted()) return;
    const request = ++generation;
    busy = true;
    error = '';
    ready = false;
    try {
      if (await authorizeEnrollment(api, principalId, () => active(request))) ready = true;
    } catch (failure) {
      if (active(request)) fail(failure);
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function changed() {
    if (!ready || pending || busy || uncertain || enrollment || !admitted()) return;
    recovery.register();
    error = '';
    observed = null;
  }
  async function restore() {
    if (busy || !pending || !admitted()) return;
    const request = ++generation;
    busy = true;
    ready = false;
    error = '';
    password = '';
    try {
      const result = await recovery.restore(
        () => authorizeEnrollment(api, principalId, () => active(request)),
        (value) => {
          name = value.name;
          email = value.email;
          role = value.role;
          uncertain = value.uncertain;
          password = '';
          observed = null;
        },
      );
      if (!active(request)) return;
      if (result.status !== 'restored') throw new Error('No se pudo recuperar el alta.');
      pending = false;
      ready = true;
    } catch (failure) {
      if (active(request)) fail(failure);
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function submit(event) {
    event.preventDefault();
    if (!ready || busy || pending || uncertain || enrollment || !admitted()) return;
    let checkedName;
    try {
      checkedName = accountName(name);
    } catch (failure) {
      error = failure.message;
      return;
    }
    const request = ++generation;
    busy = true;
    error = '';
    observed = null;
    try {
      if (!(await authorizeEnrollment(api, principalId, () => active(request)))) return;
      recovery.register();
      uncertain = true;
      writing = true;
      const response = api.createUser({ email: email.trim(), name: checkedName, password, role });
      password = '';
      const value = await response;
      if (!active(request)) return;
      recovery.close();
      reset();
      enrollment = value;
    } catch (failure) {
      if (!active(request)) return;
      if (!writing || failure.status === 403) fail(failure);
      else
        error =
          'No se pudo confirmar el alta. Consulta el directorio; no se enviara otra vez este intento.';
    } finally {
      if (alive && request === generation) {
        password = '';
        writing = false;
        busy = false;
      }
    }
  }
  async function consult() {
    if (!ready || busy || !uncertain || !admitted()) return;
    const request = ++generation;
    busy = true;
    error = '';
    observed = null;
    try {
      const result = await findEnrollmentAccount(api, members, principalId, email, () =>
        active(request),
      );
      if (result && active(request)) observed = result.found;
    } catch (failure) {
      if (active(request)) fail(failure);
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function discard() {
    if (busy || !admitted()) return;
    generation++;
    recovery.close();
    reset();
    error = '';
    onreturn();
  }
  function done() {
    if (!admitted()) return;
    recovery.close();
    reset();
    error = '';
    oncomplete();
  }
  onMount(() => {
    pending = recovery.pending();
    if (!pending) authorize();
  });
  onDestroy(() => {
    if (uncertain) recovery.retain();
    alive = false;
    generation++;
    password = '';
    enrollment = null;
    recovery.dispose();
    members.dispose();
  });
</script>

<section class="card admin-panel" aria-busy={busy}>
  {#if enrollment}<Enrollment {enrollment} ondone={done} />
  {:else}
    <span class="tile-icon"><Icon name="users" size={26} /></span>
    <h2>Nuevo integrante</h2>
    <p>
      La clave y los c&#243;digos de recuperaci&#243;n se mostrar&#225;n una sola vez.
      Entr&#233;galos de forma segura al nuevo integrante.
    </p>
    {#if pending}<p class="notice">
        Hay un alta pendiente de esta cuenta. S&#243;lo se conserva nombre, correo, rol e
        intenci&#243;n; nunca contrase&#241;a ni material MFA.
      </p>
      <Button variant="secondary" disabled={busy} onclick={restore}
        >Retomar alta de integrante</Button
      >
    {:else if !ready}
      <p role="status">Comprobando permiso de alta...</p>
      {#if !busy}<Button variant="secondary" onclick={authorize}>Volver a consultar acceso</Button
        >{/if}
    {/if}
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if uncertain && !pending}
      <p class="notice">
        El resultado del alta no est&#225; confirmado. Este intento no se puede reenviar. No se
        conserva ni recupera su segundo factor.
      </p>
      {#if observed !== null}<p role="status">
          {observed
            ? 'Existe una cuenta con este correo.'
            : 'No se encontro una cuenta con este correo.'}
        </p>{/if}
      <p class="hint">
        La consulta del directorio no confirma este envio ni recupera el segundo factor.
      </p>
      <Button variant="secondary" disabled={busy || !ready} onclick={consult}
        >Consultar directorio para esta cuenta</Button
      >
    {/if}
    <form class="stack" onsubmit={submit}>
      <MemberEnrollmentFields
        bind:name
        bind:email
        bind:password
        bind:role
        disabled={!ready || busy || pending || uncertain}
        onchanged={changed}
      />
      <Button variant="primary" disabled={!ready || busy || pending || uncertain}
        >{writing ? 'Creando...' : 'Crear usuario'}</Button
      >
    </form>
    {#if pending || uncertain || name || email || role !== 'paralegal'}<div class="action-row">
        <Button variant="secondary" disabled={busy} onclick={discard}
          >Descartar alta y volver al directorio</Button
        >
      </div>{/if}
  {/if}
</section>
