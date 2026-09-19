<script>
  import DeadlineCollection from './DeadlineCollection.svelte';
  import { canDeadlines } from '../lib/deadline-errors.mjs';
  import '../styles/procedural-facts.css';
  import '../styles/judicial-calendars.css';
  import '../styles/deadlines.css';
  export let api, user, record, ondenied, onnavigate;
  export let intent = null,
    onintent = () => {};
</script>

{#if canDeadlines(user.role)}
  <div class="page-heading">
    <div>
      <span class="eyebrow">GESTI&#211;N DEL EXPEDIENTE</span>
      <h1>Plazos del expediente</h1>
      <p>Consulta el c&#243;mputo, sus fuentes y la atenci&#243;n declarada.</p>
    </div>
  </div>
  <DeadlineCollection {api} {user} caseId={record.id} {ondenied} {intent} {onintent} />
  <button class="text-button" onclick={() => onnavigate('agenda')}>Ir a Agenda</button>
{/if}
