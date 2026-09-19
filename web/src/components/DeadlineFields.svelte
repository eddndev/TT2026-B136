<script>
  import { onMount, onDestroy } from 'svelte';
  import DeadlineFieldsBoolean from './DeadlineFieldsBoolean.svelte';
  import DeadlineFieldsSource from './DeadlineFieldsSource.svelte';
  import DeadlineCatalogPicker from './DeadlineCatalogPicker.svelte';
  import DeadlineResponsiblePicker from './DeadlineResponsiblePicker.svelte';
  import DeadlinePolicyChoice from './DeadlinePolicyChoice.svelte';
  import {
    initialDeadlinePolicies,
    reconcileDeadlinePolicies,
  } from '../lib/deadline-editor-policies.mjs';
  import { deadlineDenied, deadlineFailure } from '../lib/deadline-errors.mjs';
  export let value,
    api,
    caseId,
    ondenied,
    responsible = null,
    profile = null,
    policies = null,
    disabled = false,
    pending = false;
  const scoped = api.deadlineProfiles(caseId);
  let choosing = '',
    pickerBusy = false,
    sourceBusy = false,
    loading = false,
    alive = true,
    error = '',
    calendarName = '',
    profileKey = '';
  $: pending = pickerBusy || sourceBusy || loading;
  $: locked = disabled || pending;
  $: synchronizePolicies(value, policies);
  function synchronizePolicies(definition, current) {
    const next = current
      ? reconcileDeadlinePolicies(current, definition)
      : initialDeadlinePolicies(definition);
    if (JSON.stringify(next) !== JSON.stringify(current)) policies = next;
  }
  async function loadProfile() {
    if (!value.profile?.id || loading) return;
    loading = true;
    error = '';
    try {
      const row = await scoped.revision(value.profile.id, value.profile.revision);
      if (alive) {
        profile = row;
        profileKey = `${row.id}:${row.revision}`;
      }
    } catch (failure) {
      if (alive) {
        error = deadlineFailure(failure);
        if (deadlineDenied(failure)) ondenied(failure);
      }
    } finally {
      if (alive) loading = false;
    }
  }
  function chooseProfile(row) {
    const key = `${row.id}:${row.revision}`;
    if (key !== profileKey)
      value = {
        ...value,
        input: {
          ...value.input,
          ordered_quantity:
            profile?.definition.template.kind === 'ordered' &&
            row.definition.template.kind === 'ordered' &&
            JSON.stringify(profile.definition.template.unit) ===
              JSON.stringify(row.definition.template.unit)
              ? value.input.ordered_quantity
              : null,
          qualification: {
            ...value.input.qualification,
            scope_applies: { kind: '' },
            unresolved_incident: { kind: '' },
            conditions: row.definition.conditions.map((condition) => ({
              id: condition.id,
              applies: { kind: '' },
              locator: '',
            })),
          },
        },
      };
    value = { ...value, profile: { id: row.id, revision: row.revision } };
    profile = row;
    profileKey = key;
    choosing = '';
  }
  const unitLabel = (unit) =>
    ({ days: 'dias', civil_months: 'meses civiles', elapsed_hours: 'horas transcurridas' })[unit] ||
    unit;
  $: template = profile?.definition.template;
  $: missingConditions =
    profile?.definition.conditions.filter(
      (condition) => !value.input.qualification.conditions.some((row) => row.id === condition.id),
    ) || [];
  function addCondition(condition) {
    value = {
      ...value,
      input: {
        ...value.input,
        qualification: {
          ...value.input.qualification,
          conditions: [
            ...value.input.qualification.conditions,
            { id: condition.id, applies: { kind: '' }, locator: '' },
          ],
        },
      },
    };
  }
  onMount(() => loadProfile());
  onDestroy(() => {
    alive = false;
    pending = false;
    scoped.dispose();
  });
</script>

<div class="hearing-fields fact-fields">
  {#if error}<p class="notice error" role="alert">{error}</p>
    <button type="button" class="secondary" disabled={locked} onclick={loadProfile}
      >Consultar perfil seleccionado</button
    >{/if}
  <label
    >T&#237;tulo del plazo<input
      maxlength="200"
      disabled={locked}
      bind:value={value.title}
    /></label
  >
  <section class="case-comparison" aria-label="Perfil seleccionado">
    <h3>Regla aplicable</h3>
    {#if value.profile}<p>
        {profile?.definition.title || 'Perfil exacto seleccionado'} / Revisi&#243;n {value.profile
          .revision}
      </p>{/if}
    {#if profile}<p class="case-multiline">{profile.definition.description}</p>
      <p>
        {profile.scope.kind === 'case'
          ? 'Perfil privado del expediente'
          : `Perfil global: ${profile.scope.value.title}`}
      </p>
      {#if template.kind === 'fixed'}<p>
          Duraci&#243;n fija: {template.rule.quantity}
          {unitLabel(template.rule.kind)}.
        </p>
      {:else}<p>
          Duraci&#243;n ordenada en {unitLabel(template.unit.kind)}. M&#225;ximo declarado: {template.maximum ??
            'Sin maximo'}.
        </p>{/if}
      <p class="hint">
        El perfil se selecciona expresamente; su disponibilidad no acredita su aplicabilidad a este
        expediente.
      </p>
    {/if}
    <button type="button" class="secondary" disabled={locked} onclick={() => (choosing = 'profile')}
      >Elegir perfil exacto</button
    >
    <DeadlinePolicyChoice
      dependency="profile"
      bind:value={policies.profile.value}
      present={!!policies.profile.key}
      disabled={locked}
    />
  </section>
  <section class="case-comparison" aria-label="Responsable seleccionado">
    <h3>Responsable</h3>
    {#if responsible}<p>{responsible.email} / {responsible.role}</p>
    {:else}<p>Selecciona una persona con acceso vigente al expediente.</p>{/if}
    <button
      type="button"
      class="secondary"
      disabled={locked}
      onclick={() => (choosing = 'responsible')}>Elegir responsable</button
    >
  </section>
  {#if choosing === 'profile'}<DeadlineCatalogPicker
      {api}
      {caseId}
      {ondenied}
      disabled={disabled || sourceBusy || loading}
      bind:busy={pickerBusy}
      onselected={chooseProfile}
      oncancel={() => (choosing = '')}
    />
  {:else if choosing === 'calendar'}<DeadlineCatalogPicker
      {api}
      {caseId}
      {ondenied}
      family="calendar"
      disabled={disabled || sourceBusy || loading}
      bind:busy={pickerBusy}
      onselected={(row) => {
        value = {
          ...value,
          input: { ...value.input, calendar: { id: row.id, revision: row.revision } },
        };
        calendarName = row.values.scope.title;
        choosing = '';
      }}
      oncancel={() => (choosing = '')}
    />
  {:else if choosing === 'responsible'}<DeadlineResponsiblePicker
      {api}
      {caseId}
      {ondenied}
      disabled={disabled || sourceBusy || loading}
      bind:busy={pickerBusy}
      onselected={(row) => {
        value = { ...value, responsible_id: row.id };
        responsible = row;
        choosing = '';
      }}
      oncancel={() => (choosing = '')}
    />{/if}
  <DeadlineFieldsSource
    bind:value={value.input.selection}
    {api}
    {caseId}
    {ondenied}
    disabled={disabled || pickerBusy || loading}
    bind:pending={sourceBusy}
    bind:policy={policies.source.value}
    policyPresent={!!policies.source.key}
  />
  <section class="case-comparison" aria-label="Calendario seleccionado">
    <h3>Calendario</h3>
    {#if value.input.calendar}<p>
        {calendarName || 'Calendario exacto seleccionado'} / Revisi&#243;n {value.input.calendar
          .revision}
      </p>
    {:else}<p>
        Sin calendario seleccionado. Si la regla lo requiere, el c&#225;lculo quedar&#225;
        bloqueado.
      </p>{/if}
    <div class="action-row">
      <button
        type="button"
        class="secondary"
        disabled={locked}
        onclick={() => (choosing = 'calendar')}>Elegir calendario exacto</button
      >
      {#if value.input.calendar}<button
          type="button"
          class="text-button"
          disabled={locked}
          onclick={() => {
            value = { ...value, input: { ...value.input, calendar: null } };
            calendarName = '';
          }}>Declarar calendario ausente</button
        >{/if}
    </div>
    <DeadlinePolicyChoice
      dependency="calendar"
      bind:value={policies.calendar.value}
      present={!!policies.calendar.key}
      disabled={locked}
    />
  </section>
  <fieldset class="case-offenses" disabled={locked}>
    <legend>Duraci&#243;n ordenada</legend>
    <label
      >Cantidad ordenada<select
        value={value.input.ordered_quantity === null ? 'absent' : 'known'}
        onchange={(event) =>
          (value = {
            ...value,
            input: {
              ...value.input,
              ordered_quantity: event.currentTarget.value === 'absent' ? null : undefined,
            },
          })}
      >
        <option value="absent">Cantidad ordenada ausente</option><option value="known"
          >Cantidad declarada expresamente</option
        >
      </select></label
    >
    {#if value.input.ordered_quantity !== null}<label
        >Cantidad declarada<input
          type="number"
          min="1"
          max="4294967295"
          step="1"
          bind:value={value.input.ordered_quantity}
        /></label
      >{/if}
    <p class="hint">
      No se sustituye la duraci&#243;n concedida por el m&#225;ximo del perfil. Una cantidad ausente
      permanece ausente.
    </p>
  </fieldset>
  <fieldset class="case-offenses" disabled={locked}>
    <legend>Calificaci&#243;n del supuesto</legend>
    <label
      >Declaraci&#243;n de aplicabilidad<textarea
        rows="3"
        maxlength="1000"
        bind:value={value.input.qualification.statement}></textarea></label
    >
    <label
      >Localizador de aplicabilidad<input
        maxlength="200"
        bind:value={value.input.qualification.locator}
      /></label
    >
    <DeadlineFieldsBoolean
      bind:value={value.input.qualification.scope_applies}
      label="El ambito del perfil aplica"
      disabled={locked}
    />
    <DeadlineFieldsBoolean
      bind:value={value.input.qualification.unresolved_incident}
      label="Existe una incidencia sin resolver"
      disabled={locked}
    />
    {#each value.input.qualification.conditions as condition, index (condition.id)}<section
        class="case-comparison"
      >
        <h4>Condici&#243;n {index + 1}</h4>
        <p class="case-multiline">
          {profile?.definition.conditions.find((row) => row.id === condition.id)?.statement ||
            condition.id}
        </p>
        <DeadlineFieldsBoolean
          bind:value={condition.applies}
          label={`Se cumple la condicion ${index + 1}`}
          disabled={locked}
        />
        <label
          >Localizador de condici&#243;n {index + 1}<input
            maxlength="200"
            bind:value={condition.locator}
          /></label
        >
      </section>
    {/each}
    {#each missingConditions as condition}<section class="case-comparison">
        <p class="case-multiline">{condition.statement}</p>
        <p>Sin declaraci&#243;n guardada.</p>
        <button
          type="button"
          class="secondary"
          disabled={locked}
          onclick={() => addCondition(condition)}>Agregar declaraci&#243;n de condici&#243;n</button
        >
      </section>{/each}
    {#if !value.input.qualification.conditions.length}<p>
        Las condiciones sin declarar pueden bloquear el computo.
      </p>{/if}
  </fieldset>
</div>
