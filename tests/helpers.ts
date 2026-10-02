import { expect, type Page } from "@playwright/test";

// Seed accounts from README; all share the same password.
export const accounts = {
  company: "company@lighthouse.test",
  investor: "investor@lighthouse.test",
} as const;

export async function login(page: Page, email: string) {
  await page.goto("/");
  await page.getByLabel("이메일").fill(email);
  await page.getByLabel("비밀번호").fill("dataroom");
  await page.getByRole("button", { name: "로그인", exact: true }).click();
  await expect(page).toHaveURL(/\/workspace\/lighthouse$/);
}

/** Unique per run so tests pass on an accumulated DB (D-33). */
export function runId() {
  return new Date().toISOString().replace(/\D/g, "").slice(0, 14);
}
