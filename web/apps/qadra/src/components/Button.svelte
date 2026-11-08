<script>
  import { buttonClass } from '../lib/button-styles.mjs';
  // Every action button of Qadra, so their look lives in one place.
  // primary: the main action of a view or form; secondary: any other action;
  // text: an inline action that reads like a link; icon: a square control that
  // holds only an icon and therefore needs an aria-label.
  export let variant = 'secondary';
  // Same default as a native button, so a button inside a form still submits.
  export let type = 'submit';
  // The rendered <button>, for callers that move focus to it.
  export let element = null;
  let className = '';
  export { className as class };
</script>

<button bind:this={element} {type} class={buttonClass(variant, className)} {...$$restProps}
  ><slot /></button
>
