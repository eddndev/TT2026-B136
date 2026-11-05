<script>
  // Heroicons 24px outline set (MIT), inlined at build time.
  import home from 'heroicons/24/outline/home.svg?raw';
  import briefcase from 'heroicons/24/outline/briefcase.svg?raw';
  import folder from 'heroicons/24/outline/folder.svg?raw';
  import pencil from 'heroicons/24/outline/pencil-square.svg?raw';
  import chart from 'heroicons/24/outline/chart-pie.svg?raw';
  import calendar from 'heroicons/24/outline/calendar-days.svg?raw';
  import bell from 'heroicons/24/outline/bell.svg?raw';
  import bellAlert from 'heroicons/24/outline/bell-alert.svg?raw';
  import shield from 'heroicons/24/outline/shield-check.svg?raw';
  import users from 'heroicons/24/outline/users.svg?raw';
  import audit from 'heroicons/24/outline/clipboard-document-list.svg?raw';
  import warning from 'heroicons/24/outline/exclamation-triangle.svg?raw';
  import search from 'heroicons/24/outline/magnifying-glass.svg?raw';
  import menu from 'heroicons/24/outline/bars-3.svg?raw';
  import close from 'heroicons/24/outline/x-mark.svg?raw';
  import chevron from 'heroicons/24/outline/chevron-down.svg?raw';
  import logout from 'heroicons/24/outline/arrow-right-start-on-rectangle.svg?raw';
  import help from 'heroicons/24/outline/question-mark-circle.svg?raw';
  import fingerprint from 'heroicons/24/outline/finger-print.svg?raw';
  import key from 'heroicons/24/outline/key.svg?raw';
  import badge from 'heroicons/24/outline/check-badge.svg?raw';
  import document from 'heroicons/24/outline/document-text.svg?raw';
  import arrow from 'heroicons/24/outline/arrow-right.svg?raw';
  import clock from 'heroicons/24/outline/clock.svg?raw';
  // Panel toggle drawn in the same 24px, 1.5 stroke outline style.
  const sidebar =
    '<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor"><rect x="3" y="4.5" width="18" height="15" rx="2.25"/><path stroke-linecap="round" d="M9.75 4.5v15"/></svg>';
  const icons = {
    sidebar,
    home,
    briefcase,
    folder,
    pencil,
    chart,
    calendar,
    bell,
    bellAlert,
    shield,
    users,
    audit,
    warning,
    search,
    menu,
    close,
    chevron,
    logout,
    help,
    fingerprint,
    key,
    badge,
    document,
    arrow,
    clock,
  };
  export let name;
  let className = 'size-6';
  export { className as class };
</script>

<span class={`inline-flex shrink-0 [&>svg]:size-full ${className}`} aria-hidden="true"
  >{@html icons[name] ?? ''}</span
>
