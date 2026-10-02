import type { TextKey } from "./text";

// Same rules as the server (spec 4장, D-24, D-54).
const COMMENT_MAX_CHARS = 2000;
const EVIDENCE_MAX_COUNT = 20;

/** Trims Unicode White_Space, which is what Rust `str::trim` removes. */
export function trimComment(comment: string): string {
  return comment.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, "");
}

/** Length in code points, as the server counts it. */
export function commentLength(comment: string): number {
  return [...trimComment(comment)].length;
}

export function validateComment(comment: string): TextKey | null {
  const trimmed = trimComment(comment);
  if (!trimmed) return "commentBlank";
  if ([...trimmed].length > COMMENT_MAX_CHARS) return "commentTooLong";
  if ([...trimmed].some((c) => /\p{Cc}/u.test(c) && !"\n\r\t".includes(c))) return "commentControl";
  return null;
}

export function validateEvidence(ids: readonly string[]): TextKey | null {
  if (ids.length === 0) return "evidenceRequired";
  if (ids.length > EVIDENCE_MAX_COUNT) return "evidenceTooMany";
  return null;
}
