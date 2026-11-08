<script>
  // Title row of a modal: an optional eyebrow, the title the dialog is
  // labelled by (pass its id to the Modal's aria-labelledby) and, in the slot,
  // the control that closes the modal.
  export let title;
  export let id = undefined;
  export let eyebrow = '';
</script>

<div class="mb-4 flex items-start justify-between gap-x-4">
  <div class="min-w-0">
    {#if eyebrow}<span class="eyebrow">{eyebrow}</span>{/if}
    <h2 {id} class="text-lg font-semibold text-ink">{title}</h2>
  </div>
  <slot />
</div>
