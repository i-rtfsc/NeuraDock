/** Return an external URL only when it uses a browser-safe web protocol. */
export function safeExternalUrl(value: string | null | undefined): string | null {
  if (!value) return null;
  try {
    const url = new URL(value);
    const webProtocol = url.protocol === 'http:' || url.protocol === 'https:';
    const hasCredentials = url.username.length > 0 || url.password.length > 0;
    return webProtocol && !hasCredentials ? url.toString() : null;
  } catch {
    return null;
  }
}
