import { expect, test } from "@playwright/test";
import { accounts, login, runId } from "./helpers";

// Real API and DB. A unique title per run keeps it valid on an accumulated DB (D-33).
test("company uploads a document and sees it in detail and at the top of the list", async ({
  page,
}) => {
  const title = `매출 현황 보완 ${runId()}`;
  const fileName = "revenue-update.md";
  const content = "# 매출 현황 보완\n2026년 9월 매출 600만 원";
  await login(page, accounts.company);

  await page.getByRole("link", { name: "자료 등록" }).click();
  await page.getByLabel("제목").fill(title);
  await page.getByLabel("파일 (.txt, .md)").setInputFiles({
    name: fileName,
    mimeType: "text/markdown",
    buffer: Buffer.from(content, "utf-8"),
  });
  await page.getByRole("button", { name: "등록", exact: true }).click();

  // Wait for the detail page itself; "/documents/new" would also match an id pattern.
  await expect(page.getByRole("heading", { name: title, level: 1 })).toBeVisible();
  const detailUrl = page.url();
  expect(detailUrl).not.toMatch(/\/documents\/new$/);
  await expect(page.getByText(fileName)).toBeVisible();
  await expect(page.getByText("2026년 9월 매출 600만 원")).toBeVisible();

  await page.getByRole("link", { name: "자료 목록으로" }).click();
  await expect(page.getByRole("listitem").first()).toContainText(title);

  await page.goto(detailUrl);
  await page.reload();
  await expect(page.getByRole("heading", { name: title, level: 1 })).toBeVisible();
});
