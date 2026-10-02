import type { MessageKey } from "../i18n";

// Same limits as the server (spec 4장, D-23). Lengths count code points (D-24).
const TITLE_MAX_CHARS = 200;
const FILE_NAME_MAX_CHARS = 255;
const CONTENT_MAX_BYTES = 200_000;
const QUERY_MAX_CHARS = 100;
const UTF8_BOM_BYTES = 3;

const charCount = (value: string) => [...value].length;
// Matches Rust `char::is_control` (Unicode category Cc).
const hasControl = (value: string) => /\p{Cc}/u.test(value);

export function validateQuery(query: string): MessageKey | null {
  return charCount(query.trim()) > QUERY_MAX_CHARS ? "searchTooLong" : null;
}

export function validateTitle(title: string): MessageKey | null {
  const trimmed = title.trim();
  if (!trimmed) return "titleRequired";
  if (charCount(trimmed) > TITLE_MAX_CHARS) return "titleTooLong";
  if (hasControl(trimmed)) return "titleControl";
  return null;
}

export function validateFileName(fileName: string): MessageKey | null {
  const trimmed = fileName.trim();
  const length = charCount(trimmed);
  if (length < 1 || length > FILE_NAME_MAX_CHARS || hasControl(trimmed) || /[/\\]/.test(trimmed))
    return "fileNameInvalid";
  const dot = trimmed.lastIndexOf(".");
  const extension = dot < 0 ? "" : trimmed.slice(dot + 1).toLowerCase();
  if (extension !== "txt" && extension !== "md") return "fileExtension";
  if (dot === 0) return "fileNameInvalid";
  return null;
}

export function validateContent(content: string): MessageKey | null {
  if (!content) return "contentEmpty";
  if (new TextEncoder().encode(content).length > CONTENT_MAX_BYTES) return "contentTooLarge";
  if (content.includes("\0")) return "contentNul";
  return null;
}

/** File name without its last extension, used to prefill an empty title (D-25). */
export function titleFromFileName(fileName: string): string {
  const trimmed = fileName.trim();
  const dot = trimmed.lastIndexOf(".");
  return dot > 0 ? trimmed.slice(0, dot) : trimmed;
}

/** Reads a selected file as strict UTF-8 with the BOM removed (D-25). */
export async function readDocumentFile(
  file: File,
): Promise<{ content: string } | { error: MessageKey }> {
  const nameError = validateFileName(file.name);
  if (nameError) return { error: nameError };
  // Skip reading files that cannot fit even after the BOM is removed.
  if (file.size > CONTENT_MAX_BYTES + UTF8_BOM_BYTES) return { error: "contentTooLarge" };
  let bytes: ArrayBuffer;
  try {
    bytes = await file.arrayBuffer();
  } catch {
    return { error: "fileReadError" };
  }
  let content: string;
  try {
    // `fatal` rejects invalid bytes instead of replacing them with U+FFFD;
    // the default `ignoreBOM: false` strips a leading BOM.
    content = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    return { error: "fileEncoding" };
  }
  const contentError = validateContent(content);
  return contentError ? { error: contentError } : { content };
}
