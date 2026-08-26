import { describe, expect, it } from 'vitest';

import { safeExternalUrl } from './externalUrl';

describe('safeExternalUrl', () => {
  it('accepts http and https links', () => {
    expect(safeExternalUrl('https://example.com/path')).toBe('https://example.com/path');
    expect(safeExternalUrl('http://localhost:3000')).toBe('http://localhost:3000/');
    expect(safeExternalUrl('HTTPS://EXAMPLE.COM')).toBe('https://example.com/');
  });

  it('rejects non-web protocols and malformed values', () => {
    expect(safeExternalUrl('file:///etc/passwd')).toBeNull();
    expect(safeExternalUrl('javascript:alert(1)')).toBeNull();
    expect(safeExternalUrl('https://user:secret@example.com')).toBeNull();
    expect(safeExternalUrl('not a url')).toBeNull();
    expect(safeExternalUrl('   ')).toBeNull();
    expect(safeExternalUrl(undefined)).toBeNull();
  });
});
