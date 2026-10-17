import { useCallback, useEffect, useState } from "react";
import { Pencil, Plus, Trash2 } from "lucide-react";
import {
  createChapter,
  createSubject,
  deleteChapter,
  deleteSubject,
  listChapters,
  listSubjects,
  updateChapter,
  updateChapterStatus,
  updateSubject,
  type Chapter,
  type ChapterStatus,
  type Subject,
} from "../api";
import { AddChapterForm } from "../components/AddChapterForm";
import { ChapterRow } from "../components/ChapterRow";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { ProgressBar } from "../components/ProgressBar";
import { TextPromptDialog } from "../components/TextPromptDialog";
import { messageOf, plural } from "../format";

type PromptState =
  | { kind: "newSubject" }
  | { kind: "renameSubject"; subject: Subject }
  | { kind: "renameChapter"; chapter: Chapter };

type ConfirmState =
  | { kind: "subject"; subject: Subject }
  | { kind: "chapter"; chapter: Chapter };

export function SubjectsView() {
  const [subjects, setSubjects] = useState<Subject[] | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [chapterData, setChapterData] = useState<{ subjectId: number; items: Chapter[] } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [subjectsKey, setSubjectsKey] = useState(0);
  const [chaptersKey, setChaptersKey] = useState(0);
  const [prompt, setPrompt] = useState<PromptState | null>(null);
  const [confirm, setConfirm] = useState<ConfirmState | null>(null);
  const [busyChapterId, setBusyChapterId] = useState<number | null>(null);

  // Subject list (with progress computed by the backend).
  useEffect(() => {
    let cancelled = false;
    listSubjects()
      .then((list) => {
        if (cancelled) return;
        setSubjects(list);
        setSelectedId((cur) => (cur !== null && list.some((s) => s.id === cur) ? cur : (list[0]?.id ?? null)));
        setError(null);
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(messageOf(e));
      });
    return () => {
      cancelled = true;
    };
  }, [subjectsKey]);

  // Chapters of the selected subject.
  useEffect(() => {
    if (selectedId === null) return;
    let cancelled = false;
    listChapters(selectedId)
      .then((items) => {
        if (!cancelled) setChapterData({ subjectId: selectedId, items });
      })
      .catch((e: unknown) => {
        if (!cancelled) setError(messageOf(e));
      });
    return () => {
      cancelled = true;
    };
  }, [selectedId, chaptersKey]);

  const refresh = useCallback(() => {
    setSubjectsKey((k) => k + 1);
    setChaptersKey((k) => k + 1);
  }, []);

  const selected = subjects?.find((s) => s.id === selectedId) ?? null;
  const chapters = chapterData && chapterData.subjectId === selectedId ? chapterData.items : null;

  const changeStatus = useCallback(
    async (chapter: Chapter, status: ChapterStatus) => {
      setBusyChapterId(chapter.id);
      try {
        await updateChapterStatus(chapter.id, status);
      } catch (e) {
        setError(messageOf(e));
      } finally {
        setBusyChapterId(null);
        refresh();
      }
    },
    [refresh],
  );
  const renameChapter = useCallback((chapter: Chapter) => setPrompt({ kind: "renameChapter", chapter }), []);
  const askDeleteChapter = useCallback((chapter: Chapter) => setConfirm({ kind: "chapter", chapter }), []);

  async function submitPrompt(value: string) {
    if (!prompt) return;
    if (prompt.kind === "newSubject") {
      const created = await createSubject(value);
      setSelectedId(created.id);
    } else if (prompt.kind === "renameSubject") {
      await updateSubject(prompt.subject.id, value);
    } else {
      await updateChapter(prompt.chapter.id, value);
    }
    setPrompt(null);
    refresh();
  }

  async function submitConfirm() {
    if (!confirm) return;
    if (confirm.kind === "subject") {
      await deleteSubject(confirm.subject.id);
      setSelectedId(null);
      setChapterData(null);
    } else {
      await deleteChapter(confirm.chapter.id);
    }
    setConfirm(null);
    refresh();
  }

  const promptConfig = (() => {
    switch (prompt?.kind) {
      case "newSubject":
        return { title: "New subject", label: "Subject name", initial: "", submit: "Create subject" };
      case "renameSubject":
        return { title: "Rename subject", label: "Subject name", initial: prompt.subject.name, submit: "Save" };
      case "renameChapter":
        return { title: "Rename chapter", label: "Chapter title", initial: prompt.chapter.title, submit: "Save" };
      default:
        return { title: "", label: "", initial: "", submit: "" };
    }
  })();

  const confirmConfig = (() => {
    if (confirm?.kind === "subject") {
      const s = confirm.subject;
      const parts = [`“${s.name}” and its ${plural(s.chapterCount, "chapter")} will be permanently deleted.`];
      if (s.taskCount > 0) {
        parts.push(`Its ${plural(s.taskCount, "task")} will be kept but no longer linked to a subject.`);
      }
      return { title: "Delete subject?", message: parts.join(" ") };
    }
    if (confirm?.kind === "chapter") {
      return { title: "Delete chapter?", message: `“${confirm.chapter.title}” will be permanently deleted.` };
    }
    return { title: "", message: "" };
  })();

  return (
    <>
      <header className="page-head">
        <div>
          <h1>Subjects</h1>
          <p className="muted">Track chapters and see real progress per subject.</p>
        </div>
        <button className="btn btn-primary" onClick={() => setPrompt({ kind: "newSubject" })}>
          <Plus size={16} aria-hidden="true" /> New subject
        </button>
      </header>

      {error && (
        <div role="alert" className="banner">
          <span>{error}</span>
          <button className="btn" onClick={refresh}>
            Retry
          </button>
        </div>
      )}

      {subjects === null && !error && <p className="muted">Loading subjects…</p>}

      {subjects !== null && subjects.length === 0 && (
        <section className="panel empty">
          <p>No subjects yet.</p>
          <p className="muted">Create a subject, then add its chapters to start tracking progress.</p>
          <button className="btn btn-primary" onClick={() => setPrompt({ kind: "newSubject" })}>
            <Plus size={16} aria-hidden="true" /> Create your first subject
          </button>
        </section>
      )}

      {subjects !== null && subjects.length > 0 && (
        <div className="subjects-layout">
          <nav aria-label="Subjects">
            <ul className="subject-list">
              {subjects.map((s) => (
                <li key={s.id}>
                  <button
                    className="subject-card"
                    aria-current={s.id === selectedId ? "true" : undefined}
                    onClick={() => setSelectedId(s.id)}
                  >
                    <span className="subject-name">{s.name}</span>
                    <ProgressBar percent={s.percentComplete} label={`${s.name} progress`} />
                    <span className="muted subject-sub">
                      {s.chapterCount === 0
                        ? "No chapters"
                        : `${s.completed}/${plural(s.chapterCount, "chapter")} · ${s.percentComplete}%`}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </nav>

          {selected && (
            <section className="subject-detail" aria-label={`${selected.name} details`}>
              <div className="detail-head">
                <h2>{selected.name}</h2>
                <div className="detail-actions">
                  <button className="btn" onClick={() => setPrompt({ kind: "renameSubject", subject: selected })}>
                    <Pencil size={14} aria-hidden="true" /> Rename
                  </button>
                  <button className="btn" onClick={() => setConfirm({ kind: "subject", subject: selected })}>
                    <Trash2 size={14} aria-hidden="true" /> Delete
                  </button>
                </div>
              </div>

              <div className="summary">
                <div className="summary-percent">{selected.percentComplete}%</div>
                <div className="summary-main">
                  <ProgressBar percent={selected.percentComplete} label={`${selected.name} completion`} />
                  <p className="muted">
                    {selected.chapterCount === 0
                      ? "Add chapters to start tracking progress."
                      : `${selected.completed} of ${plural(selected.chapterCount, "chapter")} completed`}
                  </p>
                  <p className="muted breakdown">
                    {selected.notStarted} not started · {selected.inProgress} in progress ·{" "}
                    {selected.needsRevision} needs revision
                  </p>
                  <p className="muted breakdown">
                    {plural(selected.openTasks, "open task")} of {plural(selected.taskCount, "task")}
                  </p>
                </div>
              </div>

              <AddChapterForm
                onAdd={async (title) => {
                  await createChapter(selected.id, title);
                  refresh();
                }}
              />

              {chapters === null && <p className="muted">Loading chapters…</p>}
              {chapters !== null && chapters.length === 0 && (
                <p className="muted empty-note">No chapters yet. Add the first one above.</p>
              )}
              {chapters !== null && chapters.length > 0 && (
                <ul className="chapter-list" aria-label={`Chapters of ${selected.name}`}>
                  {chapters.map((c) => (
                    <ChapterRow
                      key={c.id}
                      chapter={c}
                      busy={busyChapterId === c.id}
                      onStatus={changeStatus}
                      onRename={renameChapter}
                      onDelete={askDeleteChapter}
                    />
                  ))}
                </ul>
              )}
            </section>
          )}
        </div>
      )}

      <TextPromptDialog
        open={prompt !== null}
        title={promptConfig.title}
        label={promptConfig.label}
        initialValue={promptConfig.initial}
        submitLabel={promptConfig.submit}
        onCancel={() => setPrompt(null)}
        onSubmit={submitPrompt}
      />

      <ConfirmDialog
        open={confirm !== null}
        title={confirmConfig.title}
        message={confirmConfig.message}
        confirmLabel="Delete"
        onCancel={() => setConfirm(null)}
        onConfirm={submitConfirm}
      />
    </>
  );
}
