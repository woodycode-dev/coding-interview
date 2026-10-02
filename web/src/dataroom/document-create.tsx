import { useId, useRef, useState, type ChangeEvent, type FormEvent } from "react";
import { Link, useNavigate } from "react-router-dom";
import { Button, Input, Label } from "@biyard/components";
import { useI18n, type MessageKey } from "../i18n";
import { dataroomPath, errorStatus, useCreateDocument } from "./api";
import { readDocumentFile, titleFromFileName, validateTitle } from "./document-input";

export function DocumentCreate({
  workspaceId,
  canUpload,
}: {
  workspaceId: string;
  canUpload: boolean;
}) {
  const { t } = useI18n();
  if (!canUpload)
    return (
      <div className="space-y-4">
        <p role="alert">{t.noUploadPermission}</p>
        <Link
          to={dataroomPath(workspaceId)}
          className="inline-flex min-h-11 items-center underline"
        >
          {t.backToList}
        </Link>
      </div>
    );
  return <DocumentCreateForm workspaceId={workspaceId} />;
}

function submitErrorMessage(error: unknown): MessageKey {
  const status = errorStatus(error);
  if (status === 400) return "invalidInput";
  if (status === 403) return "forbidden";
  return "uploadError";
}

function DocumentCreateForm({ workspaceId }: { workspaceId: string }) {
  const { t } = useI18n();
  const navigate = useNavigate();
  const create = useCreateDocument(workspaceId);
  const titleId = useId();
  const fileId = useId();
  const titleRef = useRef<HTMLInputElement>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  // Form state stays in component memory only (D-30).
  const [title, setTitle] = useState("");
  const [file, setFile] = useState<{ name: string; content: string } | null>(null);
  const [reading, setReading] = useState(false);
  const [titleError, setTitleError] = useState<MessageKey | null>(null);
  const [fileError, setFileError] = useState<MessageKey | null>(null);
  const busy = reading || create.isPending;

  async function selectFile(event: ChangeEvent<HTMLInputElement>) {
    const selected = event.target.files?.[0];
    setFile(null);
    setFileError(null);
    if (!selected) return;
    setReading(true);
    const result = await readDocumentFile(selected);
    setReading(false);
    if ("error" in result) {
      setFileError(result.error);
      return;
    }
    setFile({ name: selected.name.trim(), content: result.content });
    setTitle((current) => (current.trim() ? current : titleFromFileName(selected.name)));
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    const nextTitleError = validateTitle(title);
    const nextFileError = file ? null : (fileError ?? "fileRequired");
    setTitleError(nextTitleError);
    setFileError(nextFileError);
    if (nextTitleError) titleRef.current?.focus();
    else if (nextFileError) fileRef.current?.focus();
    if (nextTitleError || !file) return;
    create.mutate(
      { title: title.trim(), fileName: file.name, content: file.content },
      { onSuccess: (document) => navigate(dataroomPath(workspaceId, "documents", document.id)) },
    );
  }

  return (
    <form onSubmit={submit} noValidate className="max-w-2xl space-y-6">
      <h1 className="text-heading-4 font-semibold">{t.uploadDocument}</h1>
      <div className="flex flex-col gap-2">
        <Label htmlFor={titleId}>{t.title}</Label>
        <Input
          ref={titleRef}
          id={titleId}
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          disabled={create.isPending}
          aria-invalid={titleError ? true : undefined}
          aria-describedby={titleError ? `${titleId}-error` : undefined}
          className="min-h-11"
        />
        {titleError ? (
          <p id={`${titleId}-error`} className="text-destructive">
            {t[titleError]}
          </p>
        ) : null}
      </div>
      <div className="flex flex-col gap-2">
        <Label htmlFor={fileId}>{t.file}</Label>
        <input
          ref={fileRef}
          id={fileId}
          type="file"
          accept=".txt,.md"
          onChange={(event) => void selectFile(event)}
          disabled={busy}
          aria-invalid={fileError ? true : undefined}
          aria-describedby={`${fileId}-hint${fileError ? ` ${fileId}-error` : ""}`}
          className="min-h-11 text-body-sm"
        />
        <p id={`${fileId}-hint`} className="text-caption text-muted-foreground">
          {t.fileHint}
        </p>
        {reading ? (
          <p role="status" className="text-muted-foreground">
            {t.readingFile}
          </p>
        ) : null}
        {fileError ? (
          <p id={`${fileId}-error`} role="alert" className="text-destructive">
            {t[fileError]}
          </p>
        ) : null}
      </div>
      {create.isError ? (
        <p role="alert" className="text-destructive">
          {t[submitErrorMessage(create.error)]}
        </p>
      ) : null}
      <div className="flex flex-wrap gap-2">
        <Button type="submit" disabled={busy} className="min-h-11">
          {create.isPending ? t.submitting : t.submit}
        </Button>
        <Button asChild variant="outline" className="min-h-11">
          <Link to={dataroomPath(workspaceId)}>{t.cancel}</Link>
        </Button>
      </div>
    </form>
  );
}
