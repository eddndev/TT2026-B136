export function requestCompletion(page, matches) {
  return new Promise((resolve) => {
    const finish = (request, failed) => {
      if (!matches(request)) return;
      page.off('requestfinished', succeeded);
      page.off('requestfailed', cancelled);
      resolve({ request, failed });
    };
    const succeeded = (request) => finish(request, false);
    const cancelled = (request) => finish(request, true);
    page.on('requestfinished', succeeded);
    page.on('requestfailed', cancelled);
  });
}
