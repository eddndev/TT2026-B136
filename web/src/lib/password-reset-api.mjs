// Public recovery requests never attach an authenticated session or replay a POST.
export function createPasswordResetApi(fetcher = globalThis.fetch) {
  async function post(action, data) {
    try {
      return await fetcher(`/api/v1/auth/password-reset/${action}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
        cache: 'no-store',
        credentials: 'omit',
        redirect: 'error',
        referrerPolicy: 'no-referrer',
      });
    } catch {
      return null;
    }
  }
  return {
    async request(email) {
      const response = await post('request', { email });
      return response?.status === 202 ? 'accepted' : 'unavailable';
    },
    async complete(token, newPassword) {
      const response = await post('complete', { token, new_password: newPassword });
      if (response?.status === 204) return 'changed';
      return response?.status === 400 ? 'rejected' : 'uncertain';
    },
  };
}
