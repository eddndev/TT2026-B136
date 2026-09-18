<script>
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import DeadlineDayTrace from './DeadlineDayTrace.svelte';
  export let trace;
</script>

<ol class="deadline-trace">
  {#each trace as step}<li class="case-comparison">
      {#if step.kind === 'natural_days'}<h4>Conteo de d&#237;as naturales</h4>
        <p>Primer d&#237;a incluido: {step.first_included}. Cantidad: {step.quantity}.</p>
        <p>
          {step.candidate
            ? `Fecha candidata: ${step.candidate}`
            : 'Sin fecha candidata en el intervalo admitido.'}
        </p>
      {:else if step.kind === 'civil_months'}<h4>Adici&#243;n de meses civiles</h4>
        <p>Fecha de inicio: {step.anchor}. Cantidad: {step.quantity}.</p>
        <p>
          Mes de destino: {step.target_year}-{String(step.target_month).padStart(2, '0')}. D&#237;a
          requerido: {step.requested_day}.
        </p>
        <p>
          {step.candidate
            ? `Fecha candidata: ${step.candidate}`
            : 'El d\u00eda requerido no existe en el mes de destino.'}
        </p>
      {:else if step.kind === 'elapsed_hours'}<h4>Adici&#243;n de horas transcurridas</h4>
        <p>Inicio: {deadlineInstantLabel(step.start)}.</p>
        <p>Cantidad: {step.quantity} horas.</p>
        <p>
          {step.candidate
            ? `Instante candidato: ${deadlineInstantLabel(step.candidate)}`
            : 'Sin instante candidato en el intervalo admitido.'}
        </p>
      {:else}<h4>
          {step.kind === 'final_day'
            ? 'Revisi\u00f3n del d\u00eda final'
            : 'Conteo seg\u00fan calendario'}
        </h4>
        <DeadlineDayTrace count={step.count} />{/if}
    </li>{/each}
</ol>
