import { createContext, useContext, useState, type ReactNode } from "react";
import type { Locale } from "@interview/plugin-sdk";
const en = {
  appName: "Dataroom",
  login: "Sign in",
  logout: "Sign out",
  email: "Email",
  password: "Password",
  demoNotice: "Sign in to your workspace.",
  companyAccount: "Company account",
  investorAccount: "Investor account",
  company: "Company",
  investor: "Investor",
  language: "Language",
  loading: "Loading…",
  connectionError: "Could not connect to the API. Check the server and try again.",
  retry: "Try again",
  loginError: "Could not sign in. Check your credentials, then try again.",
  logoutError: "Could not sign out. Please try again.",
  workspace: "Workspace",
  noWorkspace: "No workspace is available for this account.",
  noPlugin: "This plugin is unavailable.",
  pluginError: "Could not load the plugin. Check the bundle and try again.",
  room: "Data room",
  menu: "Plugins",
  documents: "Documents",
  uploadDocument: "Upload document",
  searchLabel: "Search titles",
  search: "Search",
  clearSearch: "Clear search",
  searchTooLong: "Search terms must be 100 characters or fewer.",
  noDocuments: "No documents have been uploaded.",
  noSearchResults: "No documents match your search.",
  statusReady: "Ready",
  statusProcessing: "Processing",
  statusFailed: "Failed",
  fileName: "File name",
  createdAt: "Uploaded at",
  createdBy: "Uploaded by",
  content: "Content",
  documentNotFound: "Document not found.",
  backToList: "Back to documents",
  pageNotFound: "Page not found.",
  noUploadPermission: "You do not have permission to upload documents.",
  title: "Title",
  file: "File (.txt, .md)",
  fileHint: "UTF-8 encoded .txt or .md file, up to 200KB.",
  readingFile: "Reading the file…",
  submit: "Upload",
  submitting: "Uploading…",
  cancel: "Cancel",
  uploadError: "Could not upload the document. Please try again.",
  invalidInput: "Some input is invalid. Check the fields and try again.",
  forbidden: "You do not have permission for this action.",
  titleRequired: "Enter a title.",
  titleTooLong: "Titles must be 200 characters or fewer.",
  titleControl: "Titles cannot contain control characters.",
  fileRequired: "Choose a file.",
  fileNameInvalid: "File names must be 1-255 characters without / \\ or control characters.",
  fileExtension: "Only .txt or .md files can be uploaded.",
  fileEncoding: "This file is not valid UTF-8.",
  fileReadError: "Could not read the file. Choose it again.",
  contentEmpty: "The file is empty.",
  contentTooLarge: "File content must be 200KB or smaller.",
  contentNul: "Files containing NUL characters cannot be uploaded.",
};
type Strings = Record<keyof typeof en, string>;
export type MessageKey = keyof Strings;
const ko: Strings = {
  appName: "Dataroom",
  login: "로그인",
  logout: "로그아웃",
  email: "이메일",
  password: "비밀번호",
  demoNotice: "워크스페이스에 로그인해주세요.",
  companyAccount: "기업 계정",
  investorAccount: "투자자 계정",
  company: "기업 담당자",
  investor: "투자자",
  language: "언어",
  loading: "불러오는 중입니다.",
  connectionError: "API에 연결하지 못했습니다. 서버를 확인하고 다시 시도해주세요.",
  retry: "다시 시도",
  loginError: "로그인하지 못했습니다. 입력한 정보를 확인하고 다시 시도해주세요.",
  logoutError: "로그아웃하지 못했습니다. 다시 시도해주세요.",
  workspace: "워크스페이스",
  noWorkspace: "접근할 수 있는 워크스페이스가 없습니다.",
  noPlugin: "이 플러그인에 접근할 수 없습니다.",
  pluginError: "플러그인을 불러오지 못했습니다. 번들을 확인하고 다시 시도해주세요.",
  room: "데이터룸",
  menu: "플러그인",
  documents: "자료",
  uploadDocument: "자료 등록",
  searchLabel: "제목 검색",
  search: "검색",
  clearSearch: "검색 초기화",
  searchTooLong: "검색어는 100자 이하로 입력해주세요.",
  noDocuments: "등록된 자료가 없습니다.",
  noSearchResults: "검색 결과가 없습니다.",
  statusReady: "준비 완료",
  statusProcessing: "처리 중",
  statusFailed: "처리 실패",
  fileName: "파일명",
  createdAt: "등록 시각",
  createdBy: "등록자",
  content: "본문",
  documentNotFound: "자료를 찾을 수 없습니다.",
  backToList: "자료 목록으로",
  pageNotFound: "페이지를 찾을 수 없습니다.",
  noUploadPermission: "자료 등록 권한이 없습니다.",
  title: "제목",
  file: "파일 (.txt, .md)",
  fileHint: "UTF-8로 저장한 .txt 또는 .md 파일, 최대 200KB",
  readingFile: "파일을 읽는 중입니다.",
  submit: "등록",
  submitting: "등록 중입니다.",
  cancel: "취소",
  uploadError: "자료를 등록하지 못했습니다. 다시 시도해주세요.",
  invalidInput: "입력값이 올바르지 않습니다. 내용을 확인하고 다시 시도해주세요.",
  forbidden: "이 작업을 할 권한이 없습니다.",
  titleRequired: "제목을 입력해주세요.",
  titleTooLong: "제목은 200자 이하로 입력해주세요.",
  titleControl: "제목에 제어 문자를 넣을 수 없습니다.",
  fileRequired: "파일을 선택해주세요.",
  fileNameInvalid: "파일명은 1~255자이며 / \\ 와 제어 문자를 넣을 수 없습니다.",
  fileExtension: "확장자가 .txt 또는 .md인 파일만 등록할 수 있습니다.",
  fileEncoding: "UTF-8로 읽을 수 없는 파일입니다.",
  fileReadError: "파일을 읽지 못했습니다. 다시 선택해주세요.",
  contentEmpty: "본문이 비어 있는 파일입니다.",
  contentTooLarge: "파일 본문은 200KB 이하여야 합니다.",
  contentNul: "NUL 문자가 들어 있는 파일은 등록할 수 없습니다.",
};
const Context = createContext<{
  locale: Locale;
  setLocale(value: Locale): void;
  t: Strings;
} | null>(null);
export function LocaleProvider({ children }: { children: ReactNode }) {
  const [locale, setValue] = useState<Locale>("ko");
  return (
    <Context.Provider
      value={{
        locale,
        t: locale === "ko" ? ko : en,
        setLocale(value) {
          document.documentElement.lang = value;
          setValue(value);
        },
      }}
    >
      {children}
    </Context.Provider>
  );
}
export function useI18n() {
  const context = useContext(Context);
  if (!context) throw new Error("LocaleProvider is missing");
  return context;
}
