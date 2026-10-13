<script>
  import { needsCredential } from '../lib/typed-participant-fields.mjs';
  import { candidateKey } from '../lib/typed-participant-values.mjs';
  import TypedParticipantForm from './TypedParticipantForm.svelte';
  import ParticipantCredential from './ParticipantCredential.svelte';
  import ParticipantCandidates from './ParticipantCandidates.svelte';
  import TypedParticipantNotice from './TypedParticipantNotice.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  export let state, value, actions, api, manualApi, docs, caseId, generation;
  export let closed, admitted, supportContext, discardPath, ondenied, close, retry;
  export let pending = false;
  let form,
    credential,
    formBusy = false,
    credentialBusy = false;
  let candidatesBusy = false,
    directoryBusy = false;
  $: pending = state.busy || formBusy || credentialBusy || candidatesBusy || directoryBusy;
  $: natural = (state.selected?.values.kind || state.subject.kind) === 'natural_person';
  $: basis = JSON.stringify({
    subject: state.subject,
    selected: state.selected,
    role: state.role,
    base: state.original?.revision,
  });
  $: reviewBasis = JSON.stringify({ reason: state.reason, decisions: state.decisions });
  $: stale = Object.keys(state.decisions).filter(
    (key) =>
      state.review && !state.review.candidates.some((row) => candidateKey(row.reference) === key),
  );
  export function captureForm() {
    return form?.captureDraft() ?? null;
  }
  export function captureCredential() {
    return credential?.captureDraft() ?? null;
  }
  export function credentialControl() {
    return credential;
  }
  export function restoreCredential(snapshot, valid) {
    return credential
      ? credential.restoreDraft(snapshot, valid)
      : Promise.resolve(snapshot === null);
  }
</script>

<div class="stack">
  <p>
    Registra una identidad y su participaci&#243;n en el expediente. Esto no crea una cuenta ni
    acredita efectos jur&#237;dicos autom&#225;ticos.
  </p>
  {#key `${state.ownerEpoch}:${state.blocked}`}<TypedParticipantForm
      bind:this={form}
      {api}
      {docs}
      {caseId}
      bind:subject={state.subject}
      bind:selected={state.selected}
      bind:role={state.role}
      fixedSubject={state.intent === 'replace'}
      draft={state.formDraft}
      {supportContext}
      {discardPath}
      canApply={admitted}
      {ondenied}
      disabled={state.busy || credentialBusy || candidatesBusy || directoryBusy || closed}
      bind:pending={formBusy}
    />{/key}
  {#if natural}<ParticipantCredential
      bind:this={credential}
      mandatory={needsCredential(state.role.profile.kind)}
      scopeKey={`${caseId}:${generation}:${state.ownerEpoch}:${state.selected?.id || 'new-identity'}`}
      basisKey={`${basis}:${reviewBasis}`}
      disabled={state.busy || formBusy || candidatesBusy || directoryBusy || closed}
      bind:pending={credentialBusy}
      bind:value
    />{/if}
  <button
    type="button"
    class="secondary"
    disabled={pending || closed || state.uncertain || state.conflict || state.exhausted}
    onclick={actions.reviewIdentity}>Revisar identidad y coincidencias</button
  >
  {#if state.review}<ParticipantCandidates
      result={state.review}
      bind:decisions={state.decisions}
      bind:reason={state.reason}
      {docs}
      {caseId}
      {ondenied}
      onchoose={actions.choose}
      supportContext={(key) => supportContext(['decisions', 'support'], key)}
      onredeclare={(key) => discardPath(['decisions', 'support'], key)}
      disabled={state.busy || formBusy || credentialBusy || directoryBusy || closed}
      bind:pending={candidatesBusy}
    />
    <button
      type="button"
      class="secondary"
      disabled={pending || closed || state.uncertain || state.conflict || state.exhausted}
      onclick={actions.prepare}>Preparar registro</button
    >{/if}
  {#each stale as key}<section aria-label="Decision anterior sin aplicar">
      <p>
        Esta decisi&#243;n corresponde a otra revisi&#243;n y no se aplica a los candidatos
        actuales.
      </p>
      <label
        >Motivo anterior conservado<textarea readonly value={state.decisions[key].reason}
        ></textarea></label
      >
      {#if state.decisions[key].support}<label
          >Localizador anterior conservado<textarea
            readonly
            value={state.decisions[key].support.locator}></textarea></label
        >{/if}
    </section>{/each}
  <TypedParticipantNotice
    prepared={state.prepared}
    error={state.error}
    conflict={state.conflict}
    current={state.current}
    uncertain={state.uncertain}
    checkedAbsent={state.checkedAbsent}
    {pending}
    {closed}
    onrefresh={actions.refresh}
    onusecurrent={actions.useCurrent}
    onreconcile={actions.reconcile}
    onresend={actions.resend}
    api={manualApi}
    kind={state.role.profile.kind}
    {ondenied}
    directoryDisabled={state.busy || formBusy || credentialBusy || candidatesBusy || state.blocked}
    bind:directoryBusy
  />
  {#if state.blocked}<button class="secondary" disabled={pending} onclick={retry}
      >Volver a consultar la ficha y sus soportes</button
    >{/if}
  {#if state.typedCurrent}<p role="status" class="notice">
      La ficha ya fue tipificada. Este borrador conserva la intenci&#243;n anterior y s&#243;lo
      permite lectura.
    </p>{/if}
  <CaseClosedNotice />
  <div class="dialog-actions">
    <button class="secondary" disabled={pending} onclick={close}>Cancelar</button>
    <button
      class="primary"
      disabled={pending ||
        closed ||
        state.uncertain ||
        state.conflict ||
        state.exhausted ||
        !state.prepared ||
        (!!state.prepared.declaration && !value?.ready)}
      onclick={actions.submit}>{state.busy ? 'Procesando...' : 'Confirmar registro'}</button
    >
  </div>
</div>
