import { expect, test } from "@playwright/test";
import { accounts, login, runId } from "./helpers";

// Real API and DB. desktop and mobile run in order on the same DB, so the test
// overwrites the review and checks the values it just saved (D-33).
test("investor writes a review with evidence and sees it in progress after reload", async ({
  page,
}) => {
  const comment = `매출 근거 보완 필요 ${runId()}`;
  await login(page, accounts.investor);

  await page.getByRole("link", { name: "검토" }).click();
  await expect(page.getByRole("heading", { name: "내 검토 현황" })).toBeVisible();

  await page.getByRole("button", { name: "사업 이해 작성·수정" }).click();
  await expect(page).toHaveURL(/\/plugins\/review\/criteria\/business$/);
  await expect(page.getByRole("heading", { name: "사업 이해", level: 1 })).toBeVisible();

  await page.getByRole("radio", { name: "추가 확인 필요" }).check();
  await page.getByLabel("의견").fill(comment);
  await page.getByRole("checkbox", { name: /회사 소개/ }).check();
  // Processing and failed documents cannot be chosen as evidence.
  await expect(page.getByRole("checkbox", { name: /고객 인터뷰/ })).toBeDisabled();
  await expect(page.getByRole("checkbox", { name: /매출 자료/ })).toBeDisabled();

  await page.getByRole("button", { name: "저장", exact: true }).click();

  // The plugin home path is "/", so the host URL keeps a trailing slash.
  await expect(page).toHaveURL(/\/plugins\/review\/\?saved=business$/);
  await expect(page.getByText("저장했습니다: 사업 이해")).toBeVisible();
  const row = page
    .getByRole("listitem")
    .filter({ has: page.getByRole("heading", { name: "사업 이해", level: 3 }) });
  await expect(row.getByText("추가 확인 필요", { exact: true })).toBeVisible();
  await expect(row.getByText(comment)).toBeVisible();
  await expect(row.getByRole("button", { name: "회사 소개" })).toBeVisible();

  await page.reload();
  await expect(row.getByText("추가 확인 필요", { exact: true })).toBeVisible();
  await expect(row.getByText(comment)).toBeVisible();

  // Three distinct badges (README): save "매출 현황" as satisfied; no test ever
  // writes "팀 구성", so it stays unwritten even on an accumulated DB.
  await page.getByRole("button", { name: "매출 현황 작성·수정" }).click();
  await page.getByRole("radio", { name: "확인함" }).check();
  await page.getByLabel("의견").fill(`매출 확인 ${runId()}`);
  await page.getByRole("checkbox", { name: /팀 소개/ }).check();
  await page.getByRole("button", { name: "저장", exact: true }).click();
  await expect(page.getByText("저장했습니다: 매출 현황")).toBeVisible();

  const badge = (criterion: string, label: string) =>
    page
      .getByRole("listitem")
      .filter({ has: page.getByRole("heading", { name: criterion, level: 3 }) })
      .getByText(label, { exact: true });
  const badges = [
    badge("사업 이해", "추가 확인 필요"),
    badge("매출 현황", "확인함"),
    badge("팀 구성", "미작성"),
  ];
  for (const item of badges) await expect(item).toBeVisible();
  const colors = await Promise.all(
    badges.map((item) => item.evaluate((element) => getComputedStyle(element).backgroundColor)),
  );
  expect(new Set(colors).size).toBe(3);
});
