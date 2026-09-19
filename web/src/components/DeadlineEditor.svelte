<script>
  import { onDestroy } from 'svelte';
  import DeadlineFields from './DeadlineFields.svelte';
  import DeadlineFieldsAttention from './DeadlineFieldsAttention.svelte';
  import DeadlineFieldsReview from './DeadlineFieldsReview.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  import {
    canDeadlines,
    deadlineDenied,
    deadlineFailure,
    deadlineUncertain,
  } from '../lib/deadline-errors.mjs';
  import { readDeadlineSubmission } from '../lib/deadline-submission.mjs';
  import {
    initialDeadlinePolicies,
    reconcileDeadlinePolicies,
    deadlinePoliciesCommand,
    adoptDeadlineDefinition,
  } from '../lib/deadline-editor-policies.mjs';
  export let api,
    user,
    caseId,
    base = null,
    mode = 'register',
    onsaved,
    oncancel,
    ondenied = () => {},
    pending = false,
    disabled = false;
  const administration = caseState();
  let scoped = null,
    seen = '',
    generation = 0,
    alive = true,
    current = null,
    id = '',
    definition,
    policies,
    fieldsVersion = 0,
    attention = { status: '' },
    reason = '',
    profile = null,
    responsible = null,
    busy = false,
    fieldsBusy = false,
    step = 'draft',
    prepared = null,
    last = null,
    candidate = null,
    compared = false,
    error = '',
    acknowledge = false;
  $: allowed = canDeadlines(user?.role, 'manage');
  $: identity = `${caseId}:${user?.id}:${user?.email}:${user?.role}:${mode}:${base?.id || ''}:${base?.revision || ''}`;
  $: if (identity !== seen) reset(identity);
  $: pending = busy || fieldsBusy;
  $: frozen =
    disabled ||
    pending ||
    $administration.closed ||
    !allowed ||
    ['uncertain', 'exhausted'].includes(step);
  function reset(identity) {
    seen = identity;
    generation++;
    scoped?.dispose();
    scoped = null;
    busy = false;
    fieldsBusy = false;
    step = 'draft';
    prepared = null;
    last = null;
    candidate = null;
    compared = false;
    error = '';
    acknowledge = false;
    profile = null;
    current = base;
    responsible = base?.responsible || null;
    id = base?.id || crypto.randomUUID();
    reason = '';
    attention = { status: '' };
    definition = base
      ? structuredClone(base.definition)
      : {
          title: '',
          profile: null,
          responsible_id: '',
          input: {
            selection: { case_id: caseId, source: { kind: '' }, qualification: null },
            calendar: null,
            ordered_quantity: null,
            qualification: {
              statement: '',
              locator: '',
              scope_applies: { kind: '' },
              unresolved_incident: { kind: '' },
              conditions: [],
            },
          },
        };
    policies = initialDeadlinePolicies(definition, base?.tracking?.policies);
    fieldsVersion++;
    if (canDeadlines(user?.role, 'manage')) scoped = api.deadlines(caseId);
  }
  const currentRequest = (token) => alive && token === generation;
  function fail(failure, writing = false) {
    prepared = null;
    acknowledge = false;
    error = deadlineFailure(failure);
    if (deadlineDenied(failure)) {
      ondenied(failure);
      return;
    }
    if (writing && (deadlineUncertain(failure) || failure.code === 'deadline_operation_conflict')) {
      step = 'uncertain';
      error =
        'No se pudo confirmar el resultado. Conservamos el envio para consultar su revision exacta.';
    } else if (failure.code === 'deadline_revision_exhausted') step = 'exhausted';
    else if (
      ['deadline_revision_conflict', 'deadline_retired', 'deadline_not_found'].includes(
        failure.code,
      )
    ) {
      step = 'conflict';
      candidate = null;
      compared = false;
    } else step = 'draft';
  }
  async function prepare() {
    if (frozen || step !== 'draft') return;
    const token = generation;
    busy = true;
    error = '';
    acknowledge = false;
    try {
      const change = {
        action: mode,
        expected_revision: mode === 'register' ? 0 : current.revision,
      };
      if (['register', 'correct'].includes(mode)) {
        change.definition = structuredClone(definition);
        change.tracking = deadlinePoliciesCommand(policies, definition);
      }
      if (mode === 'set_attention') change.attention = structuredClone(attention);
      if (mode !== 'register') change.reason = reason;
      const value = await scoped.prepare(
        { operation_id: crypto.randomUUID(), deadline_id: id, change },
        { id: user.id, email: user.email, role: user.role },
      );
      if (currentRequest(token)) {
        prepared = structuredClone(value);
        step = 'review';
      }
    } catch (failure) {
      if (currentRequest(token)) fail(failure);
    } finally {
      if (currentRequest(token)) busy = false;
    }
  }
  async function finish(value, exact = false) {
    prepared = null;
    step = 'confirmed';
    try {
      await onsaved(value, exact);
    } catch {
      if (alive)
        error = 'El plazo se guardo. Consulta de nuevo sin reenviar si faltan datos en pantalla.';
    }
  }
  async function submit() {
    if (frozen || step !== 'review' || !prepared || !acknowledge) return;
    const token = generation;
    busy = true;
    error = '';
    last = structuredClone(prepared);
    try {
      const value = await scoped.submit(last);
      if (currentRequest(token)) await finish(value);
    } catch (failure) {
      if (currentRequest(token)) fail(failure, true);
    } finally {
      if (currentRequest(token)) busy = false;
    }
  }
  async function check() {
    if (pending || !last || !allowed) return;
    const token = generation;
    busy = true;
    error = '';
    try {
      const value = await readDeadlineSubmission(scoped, last);
      if (!currentRequest(token)) return;
      if (value.state === 'matched') await finish(value.record, true);
      else if (value.state === 'absent')
        error =
          'La revision aun no esta disponible. El resultado sigue incierto; puedes consultar de nuevo.';
      else {
        step = 'conflict';
        candidate = value.record;
        compared = false;
        error = 'La revision pertenece a otro envio. Conservamos tu borrador.';
      }
    } catch (failure) {
      if (currentRequest(token)) {
        error = deadlineFailure(failure);
        if (deadlineDenied(failure)) ondenied(failure);
      }
    } finally {
      if (currentRequest(token)) busy = false;
    }
  }
  async function compare() {
    if (pending || !allowed) return;
    const token = generation;
    busy = true;
    error = '';
    compared = false;
    try {
      const value = await scoped.get(id);
      if (currentRequest(token)) {
        candidate = value;
        compared = true;
      }
    } catch (failure) {
      if (currentRequest(token)) {
        error = deadlineFailure(failure);
        if (deadlineDenied(failure)) ondenied(failure);
      }
    } finally {
      if (currentRequest(token)) busy = false;
    }
  }
  function accept() {
    if (frozen || !compared || !candidate || candidate.status !== 'active' || mode === 'register')
      return;
    if (mode === 'correct') {
      definition = adoptDeadlineDefinition(current.definition, definition, candidate.definition);
      if (definition.responsible_id === candidate.responsible.id)
        responsible = candidate.responsible;
      policies = reconcileDeadlinePolicies(policies, definition);
      profile = null;
      fieldsVersion++;
    }
    current = candidate;
    prepared = null;
    step = 'draft';
    compared = false;
    error = '';
    acknowledge = false;
  }
  onDestroy(() => {
    alive = false;
    generation++;
    pending = false;
    scoped?.dispose();
  });
</script>

{#if allowed}
  <section
    class="card case-editor fact-editor"
    aria-label="Formulario de plazo"
    aria-busy={pending}
  >
    <h2>
      {mode === 'register'
        ? 'Registrar plazo'
        : mode === 'correct'
          ? 'Corregir plazo'
          : mode === 'set_attention'
            ? 'Declarar atencion'
            : 'Retirar plazo'}
    </h2>
    <p class="hint">
      La captura conserva una evaluaci&#243;n de datos declarados. La revisi&#243;n de aplicabilidad
      corresponde a la persona operadora.
    </p>
    <CaseClosedNotice />
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if step === 'uncertain'}<section
        class="case-comparison"
        aria-label="Resultado incierto del plazo"
      >
        <h3>Resultado incierto</h3>
        <p>Una ausencia temporal no confirma que la escritura haya fallado.</p>
        <button class="primary" disabled={pending} onclick={check}>Consultar envio exacto</button>
        <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
      </section>{/if}
    {#if step === 'conflict'}<section class="case-comparison" aria-label="Comparar base del plazo">
        <h3>Comparar con el plazo actual</h3>
        <button class="secondary" disabled={pending} onclick={compare}>Consultar base actual</button
        >
        {#if compared && candidate}<p>
            Base consultada: revisi&#243;n {candidate.revision} / {candidate.status === 'retired'
              ? 'Retirado'
              : 'Activo'}
          </p>
          <DeadlineFieldsReview value={candidate} />
          {#if candidate.status === 'active' && mode !== 'register'}<button
              class="primary"
              disabled={frozen}
              onclick={accept}>Usar esta base y conservar borrador</button
            >
          {:else}<p>
              Esta captura no permite reenviar sobre la base consultada. Cierra el formulario para
              revisar su historia.
            </p>{/if}
        {/if}
      </section>{/if}
    {#if prepared}<section class="case-comparison" aria-label="Revision del plazo a confirmar">
        <h3>Revisa el plazo a confirmar</h3>
        <p>Revisi&#243;n a registrar: {prepared.result_revision}</p>
        <DeadlineFieldsReview value={prepared} {profile} />
        {#if prepared.command.change.reason}<p class="case-multiline">
            Motivo: {prepared.command.change.reason}
          </p>{/if}
        <label class="checkbox"
          ><input type="checkbox" bind:checked={acknowledge} disabled={frozen} />
          {prepared.calculation.result.blocks.length
            ? 'Revise las declaraciones, fuentes y politicas; confirmo guardar el plazo con estos bloqueos'
            : 'Revise las declaraciones, fuentes, politicas y el resultado a guardar'}</label
        >
        <div class="action-row">
          <button
            class="secondary"
            disabled={pending}
            onclick={() => {
              prepared = null;
              step = 'draft';
              acknowledge = false;
            }}>Volver al borrador</button
          >
          <button class="primary" disabled={frozen || !acknowledge} onclick={submit}
            >Confirmar plazo</button
          >
        </div>
      </section>{:else if step !== 'confirmed'}
      {#key `${identity}:${fieldsVersion}`}
        {#if ['register', 'correct'].includes(mode)}<DeadlineFields
            {api}
            {caseId}
            {ondenied}
            bind:value={definition}
            bind:profile
            bind:responsible
            bind:policies
            bind:pending={fieldsBusy}
            disabled={disabled ||
              busy ||
              $administration.closed ||
              ['uncertain', 'exhausted'].includes(step)}
          />
        {:else if mode === 'set_attention'}<DeadlineFieldsAttention
            bind:value={attention}
            disabled={frozen}
          />
        {:else}<p class="notice">
            El retiro es terminal. Conserva el c&#225;lculo y la historia; no anula el acto ni
            declara extinguido el plazo.
          </p>{/if}
      {/key}
      {#if mode !== 'register'}<label
          >Motivo<textarea rows="3" maxlength="1000" bind:value={reason} disabled={frozen}
          ></textarea></label
        >{/if}
      <button class="primary" disabled={frozen || step !== 'draft'} onclick={prepare}
        >Preparar plazo</button
      >
    {/if}
    <button class="text-button" disabled={pending} onclick={oncancel}>Cerrar formulario</button>
  </section>
{/if}
