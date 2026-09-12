<script>
  export let name = 'file';
  export let size = 20;
  const paths = {
    home: 'M3 10 12 3l9 7v11h-6v-7H9v7H3Z',
    file: 'M14 2H5v20h14V7Zm0 0v6h5M8 12h8M8 16h6',
    folder: 'M3 6h7l2 3h9v11H3Zm0 0V4h7l2 2h7v3',
    shield: 'M12 3 3 6v6c0 5 9 9 9 9s9-4 9-9V6Zm-4 9 3 3 5-6',
    users:
      'M16 21v-3a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v3M9 10a4 4 0 1 0 0-8 4 4 0 0 0 0 8m7-7a4 4 0 0 1 0 8m6 10v-3a4 4 0 0 0-3-4',
    upload: 'M12 16V3m-5 5 5-5 5 5M3 15v6h18v-6',
    arrow: 'M4 12h16m-6-6 6 6-6 6',
    check: 'm5 12 4 4L19 6',
    search: 'M21 21l-5-5M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0',
    lock: 'M6 10V7a6 6 0 0 1 12 0v3M4 10h16v12H4Zm8 5v3',
    logout: 'M9 3H3v18h6m4-14 5 5-5 5m-6-5h14',
    download: 'M12 3v13m-5-5 5 5 5-5M3 17v4h18v-4',
    close: 'm6 6 12 12M6 18 18 6',
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"><path d={paths[name] || paths.file} /></svg
>
