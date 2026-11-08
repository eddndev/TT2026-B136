// The look of every action button, shared by Button.svelte and by the few
// elements that only look like a button (for example the visible face of a
// file picker label). The btn-* marker classes let page styles position a
// button (width, margins) without restyling it.
const base =
  'inline-flex items-center justify-center gap-x-1.5 text-sm font-semibold disabled:cursor-not-allowed disabled:opacity-50';
const variants = {
  primary:
    'btn-primary rounded-md bg-brand-800 px-3 py-2 text-white shadow-xs enabled:hover:bg-brand-700',
  secondary:
    'btn-secondary rounded-md bg-white px-3 py-2 text-ink shadow-xs ring-1 ring-slate-300 ring-inset enabled:hover:bg-slate-50',
  text: 'btn-text rounded-sm px-1 py-0.5 text-brand-800 enabled:hover:text-brand-950',
  icon: 'btn-icon size-9 rounded-md text-slate-500 enabled:hover:bg-slate-100 enabled:hover:text-slate-700',
};

export function buttonClass(variant = 'secondary', extra = '') {
  if (!variants[variant]) throw new Error(`Unknown button variant: ${variant}`);
  return `${base} ${variants[variant]} ${extra}`.trim();
}
