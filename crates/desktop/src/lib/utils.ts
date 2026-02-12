/**
 * Utility functions for the Jax UI
 */

/** Convert a byte array (number[]) to a UTF-8 string */
export function bytesToString(bytes: number[]): string {
  return new TextDecoder().decode(new Uint8Array(bytes));
}

/** Convert a byte array to a data URL for displaying images */
export function bytesToDataUrl(bytes: number[], mime: string): string {
  const binary = String.fromCharCode(...new Uint8Array(bytes));
  const b64 = btoa(binary);
  return `data:${mime};base64,${b64}`;
}

/** Convert a byte array to a hex dump string (first maxBytes) */
export function bytesToHexDump(bytes: number[], maxBytes: number = 4096): string {
  const slice = bytes.slice(0, maxBytes);
  const lines: string[] = [];
  for (let i = 0; i < slice.length; i += 16) {
    const offset = i.toString(16).padStart(8, '0');
    const chunk = slice.slice(i, i + 16);
    const hex = chunk.map(b => b.toString(16).padStart(2, '0')).join(' ');
    const ascii = chunk.map(b => (b >= 32 && b <= 126) ? String.fromCharCode(b) : '.').join('');
    lines.push(`${offset}  ${hex.padEnd(48)}  ${ascii}`);
  }
  if (bytes.length > maxBytes) {
    lines.push(`... (${bytes.length - maxBytes} more bytes)`);
  }
  return lines.join('\n');
}

/** Check if a MIME type represents text content */
export function isTextMime(mime: string): boolean {
  if (mime.startsWith('text/')) return true;
  if (mime === 'application/json') return true;
  if (mime === 'application/xml') return true;
  return false;
}

/** Check if a MIME type represents an image */
export function isImageMime(mime: string): boolean {
  return mime.startsWith('image/');
}

/** Check if a MIME type represents video */
export function isVideoMime(mime: string): boolean {
  return mime.startsWith('video/');
}

/** Check if a MIME type represents audio */
export function isAudioMime(mime: string): boolean {
  return mime.startsWith('audio/');
}

/** Convert a byte array to a blob URL for media playback */
export function bytesToBlobUrl(bytes: number[], mime: string): string {
  const blob = new Blob([new Uint8Array(bytes)], { type: mime });
  return URL.createObjectURL(blob);
}

/** Format file size in human-readable form */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const size = bytes / Math.pow(1024, i);
  return `${size < 10 ? size.toFixed(1) : Math.round(size)} ${units[i]}`;
}

/** Split a path into breadcrumb segments */
export interface Breadcrumb {
  label: string;
  path: string;
}

export function pathToBreadcrumbs(path: string): Breadcrumb[] {
  const parts = path.split('/').filter(Boolean);
  const crumbs: Breadcrumb[] = [{ label: '/', path: '/' }];
  for (let i = 0; i < parts.length; i++) {
    crumbs.push({
      label: parts[i],
      path: '/' + parts.slice(0, i + 1).join('/'),
    });
  }
  return crumbs;
}

/** Get file extension from a path */
export function getExtension(path: string): string {
  const name = path.split('/').pop() || '';
  const dotIndex = name.lastIndexOf('.');
  if (dotIndex <= 0) return '';
  return name.slice(dotIndex + 1).toLowerCase();
}

/** Get the parent directory of a path */
export function parentPath(path: string): string {
  const parts = path.split('/').filter(Boolean);
  if (parts.length <= 1) return '/';
  return '/' + parts.slice(0, -1).join('/');
}
