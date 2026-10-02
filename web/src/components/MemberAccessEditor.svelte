<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import { roles } from '../lib/documents.mjs';
  import { memberAccessChange } from '../lib/members-values.mjs';
  import { createMemberAccessDraft, freshMemberAccess } from '../lib/member-access-draft.mjs';
  export let api, user, record, onconfirmed, oncancel, ondenied;
  const scoped = api.members(),
    session = getContext('session-drafts'),
    principalId = user.id;
  let base = structuredClone(record),
    role = '',
    status = '';
  let mode = 'blocked',
    prepared = null,
    candidate = null,
    busy = true,
    error = '',
    alive = true,
    finished = false,
    unconfirmed = false;
  const recovery = session
    ? createMemberAccessDraft({
        session,
        principalId,
        id: record.id,
        capture: () => ({ base, role, status, unconfirmed }),
      })
    : null;
  const admitted = () => alive && !finished && (!recovery || recovery.admitted());
  async function initialize(current) {
    if (!admitted()) return;
    busy = true;
    error = '';
    prepared = candidate = null;
    try {
      if (recovery?.pending()) {
        const result = await recovery.restore(current, (value) => {
          ({ base, role, status, unconfirmed } = value);
          mode = unconfirmed
            ? 'uncertain'
            : current.revision !== base.revision
              ? 'conflict'
              : 'draft';
        });
        if (!admitted()) return;
        if (result.status !== 'restored')
          throw new Error(
            'No se pudo recuperar el borrador de acceso. Vuelve a consultar la cuenta.',
          );
      } else {
        base = structuredClone(current);
        role = current.role;
        status = current.active ? 'active' : 'inactive';
        recovery?.register();
        mode = 'draft';
      }
    } catch (failure) {
      if (admitted()) {
        mode = 'blocked';
        error = failure.message;
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function fresh() {
    return freshMemberAccess({ api, members: scoped, principalId, id: record.id, admitted });
  }
  async function retry() {
    if (busy || mode !== 'blocked' || !admitted()) return;
    busy = true;
    error = '';
    try {
      const current = await fresh();
      if (current && admitted()) await initialize(current);
    } catch (failure) {
      if (admitted()) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure, record.id);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function close() {
    if (busy || !admitted()) return;
    recovery?.close();
    finished = true;
    oncancel();
  }
  function review(event) {
    event?.preventDefault();
    if (busy || mode !== 'draft' || !admitted()) return;
    try {
      prepared = memberAccessChange({
        expected_revision: base.revision,
        role,
        active: status === 'active',
      });
      error = '';
      mode = 'review';
    } catch (failure) {
      error = failure.message;
    }
  }
  function fail(failure, writing = false) {
    error = failure.message;
    prepared = null;
    candidate = null;
    if (writing) unconfirmed = !failure.status || failure.status >= 500;
    if ([403, 404].includes(failure.status)) {
      ondenied(failure, base.id);
      return;
    }
    if (failure.code === 'user_revision_conflict') mode = 'conflict';
    else if (writing && (!failure.status || failure.status >= 500)) mode = 'uncertain';
    else mode = 'draft';
  }
  async function confirm() {
    if (busy || mode !== 'review' || !prepared || !admitted()) return;
    busy = true;
    unconfirmed = true;
    error = '';
    try {
      const confirmed = await scoped.changeAccess(base.id, prepared, () => {
        if (!alive) return;
        recovery?.close();
        unconfirmed = false;
        finished = true;
        prepared = null;
        mode = 'confirmed';
      });
      if (alive) await onconfirmed(confirmed);
    } catch (failure) {
      if (admitted()) fail(failure, true);
    } finally {
      if (alive) busy = false;
    }
  }
  async function consult() {
    if (busy || !admitted() || !['conflict', 'uncertain'].includes(mode)) return;
    busy = true;
    error = '';
    candidate = null;
    try {
      const current = await fresh();
      if (!current || !admitted()) return;
      if (BigInt(current.revision) < BigInt(base.revision))
        throw new Error('La cuenta actual no confirma la revisi\u00f3n del borrador.');
      candidate = structuredClone(current);
    } catch (failure) {
      if (admitted()) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure, base.id);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (busy || !candidate || !admitted()) return;
    base = candidate;
    candidate = null;
    prepared = null;
    unconfirmed = false;
    mode = 'draft';
    error = '';
  }
  onMount(() => {
    initialize(record);
  });
  onDestroy(() => {
    alive = false;
    recovery?.dispose();
    scoped.dispose();
    prepared = null;
    candidate = null;
  });
</script>

<section class="card member-access" aria-label="Acceso de cuenta" aria-busy={busy}>
  <div class="section-heading">
    <div>
      <span class="eyebrow">PERMISOS DE LA CUENTA</span>
      <h2>{base.email}</h2>
      <p class="hint">Revisi&#243;n consultada {base.revision}</p>
    </div>
    <button class="text-button" disabled={busy} onclick={close}>Cerrar acceso de cuenta</button>
  </div>
  <p class="notice">
    Desactivar conserva las asignaciones y bloquea el acceso. Reactivar conserva esas asignaciones y
    exige un nuevo inicio de sesi&#243;n.
  </p>
  {#if user?.id === base.id}<p class="notice">
      Un cambio efectivo de tu propio rol o estado cerrar&#225; esta sesi&#243;n.
    </p>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if mode === 'blocked'}<button class="secondary" disabled={busy} onclick={retry}
      >Volver a consultar el acceso de cuenta</button
    >{/if}
  <form class="stack" onsubmit={review}>
    <div class="case-field-grid">
      <label
        >Rol de la cuenta<select bind:value={role} disabled={busy || mode !== 'draft'}>
          {#each Object.entries(roles) as [value, label]}<option {value}>{label}</option>{/each}
        </select></label
      >
      <label
        >Estado de la cuenta<select bind:value={status} disabled={busy || mode !== 'draft'}>
          <option value="active">Activa</option><option value="inactive">Inactiva</option>
        </select></label
      >
    </div>
    {#if mode === 'draft'}<button class="primary" disabled={busy}>Revisar acceso</button>{/if}
  </form>
  {#if mode === 'review'}<div class="notice">
      <h3>Confirmar los permisos seleccionados</h3>
      <p>
        {roles[base.role]} / {base.active ? 'Activa' : 'Inactiva'} a {roles[prepared.role]} / {prepared.active
          ? 'Activa'
          : 'Inactiva'}.
      </p>
      <p>
        Los cambios efectivos invalidan sesiones y desaf&#237;os anteriores. Las capturas
        hist&#243;ricas permanecen intactas.
      </p>
      <div class="action-row">
        <button class="primary" disabled={busy} onclick={confirm}>Confirmar cambio de acceso</button
        >
        <button
          class="secondary"
          disabled={busy}
          onclick={() => {
            mode = 'draft';
            prepared = null;
          }}>Editar selecci&#243;n</button
        >
      </div>
    </div>{/if}
  {#if ['conflict', 'uncertain'].includes(mode)}<div class="notice">
      <p>
        {mode === 'uncertain'
          ? 'No se pudo confirmar el resultado. Consulta la cuenta antes de realizar otro cambio.'
          : 'La revisi\u00f3n cambi\u00f3. Revisa el estado actual y decide si conservas tu selecci\u00f3n.'}
      </p>
      <button class="secondary" disabled={busy} onclick={consult}>Consultar cuenta actual</button>
      {#if candidate}<p>
          Revisi&#243;n actual {candidate.revision}: {roles[candidate.role]} / {candidate.active
            ? 'Activa'
            : 'Inactiva'}.
        </p>
        <button class="secondary" disabled={busy} onclick={accept}
          >Usar revisi&#243;n actual y conservar cambios</button
        >{/if}
    </div>{/if}
</section>

<style>
  .member-access {
    margin-block: 24px;
  }
  h2 {
    overflow-wrap: anywhere;
  }
  .section-heading {
    align-items: flex-start;
    flex-wrap: wrap;
  }
  form {
    margin-block: 20px;
  }
</style>
