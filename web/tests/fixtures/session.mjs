export function absoluteSession(user, accessToken) {
  const serverNow = Date.now();
  return {
    access_token: accessToken,
    token_type: 'Bearer',
    expires_in_seconds: 86400,
    user,
    policy: { absolute_ttl_seconds: 86400, idle_ttl_seconds: null },
    server_now_unix_ms: serverNow,
    absolute_expires_at_unix_ms: serverNow + 86400000,
    idle_expires_at_unix_ms: null,
  };
}
