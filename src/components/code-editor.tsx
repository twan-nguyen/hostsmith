import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { SearchQuery, findNext, highlightSelectionMatches, search, searchKeymap, setSearchQuery } from "@codemirror/search";
import { Compartment, EditorSelection, type Extension, Prec, RangeSetBuilder, Transaction } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
  drawSelection,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { useEffect, useImperativeHandle, useRef } from "react";

import { toggleComment, tokenize } from "@/lib/syntax";
import { useStore } from "@/store";

export type CodeEditorHandle = {
  /** Selects the next match of `query`, ignoring case, wrapping at the end. */
  find: (query: string) => void;
  /** Moves the cursor to the first line that equals `line` once trimmed. */
  revealLine: (line: string) => void;
};

const MARKS = Object.fromEntries(
  (["ip", "host", "comment", "marker"] as const).flatMap((kind) => [
    [kind, Decoration.mark({ class: `cm-tok-${kind}` })],
    [`${kind}-off`, Decoration.mark({ class: `cm-tok-${kind} cm-tok-disabled` })],
  ]),
);

/** Colours the visible lines with the tokenizer's spans. */
function highlighter() {
  const build = (view: EditorView): DecorationSet => {
    const builder = new RangeSetBuilder<Decoration>();
    for (const { from, to } of view.visibleRanges) {
      for (let pos = from; pos <= to; ) {
        const line = view.state.doc.lineAt(pos);
        for (const token of tokenize(line.text)) {
          builder.add(line.from + token.from, line.from + token.to, MARKS[token.disabled ? `${token.kind}-off` : token.kind]);
        }
        pos = line.to + 1;
      }
    }
    return builder.finish();
  };
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;
      constructor(view: EditorView) {
        this.decorations = build(view);
      }
      update(update: ViewUpdate) {
        if (update.docChanged || update.viewportChanged) this.decorations = build(update.view);
      }
    },
    { decorations: (plugin) => plugin.decorations },
  );
}

/** ⌘/ comments out the selected lines, or uncomments them. */
function toggleLineComments(view: EditorView) {
  const { state } = view;
  const changes = state.selection.ranges.flatMap((range) => {
    const first = state.doc.lineAt(range.from).number;
    const last = state.doc.lineAt(range.to).number;
    const lines = Array.from({ length: last - first + 1 }, (_, i) => state.doc.line(first + i));
    const toggled = toggleComment(lines.map((l) => l.text));
    return lines.map((line, i) => ({ from: line.from, to: line.to, insert: toggled[i] }));
  });
  view.dispatch({ changes, userEvent: "input" });
  return true;
}

const theme = EditorView.theme({
  "&": { height: "100%", backgroundColor: "transparent" },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": { fontFamily: "var(--font-mono)", lineHeight: "1.6" },
  ".cm-content": { padding: "12px 0", caretColor: "var(--foreground)" },
  ".cm-gutters": { backgroundColor: "transparent", border: "none", color: "var(--muted-foreground)" },
  ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "color-mix(in oklch, var(--foreground) 4%, transparent)" },
  ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
    backgroundColor: "color-mix(in oklch, var(--primary) 25%, transparent) !important",
  },
  ".cm-searchMatch": { backgroundColor: "color-mix(in oklch, var(--warning) 35%, transparent)" },
  ".cm-panels": { backgroundColor: "var(--card)", color: "var(--foreground)" },
  ".cm-panels.cm-panels-top": { borderBottom: "1px solid var(--border)" },
});

/**
 * The raw hosts editor. The parent owns the text: `value` replaces
 * the document when it changes from outside, and every edit is reported.
 */
export function CodeEditor({
  value,
  onChange,
  onSave,
  readOnly = false,
  testId,
  ref,
}: {
  value: string;
  onChange: (text: string) => void;
  onSave: () => void;
  readOnly?: boolean;
  testId?: string;
  ref?: React.Ref<CodeEditorHandle>;
}) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const fontSize = useStore((s) => s.settings?.editorFontSize ?? 13);
  // The keymap is built once; these refs keep it calling the latest handlers.
  const handlers = useRef({ onChange, onSave });
  handlers.current = { onChange, onSave };
  const editable = useRef(new Compartment());

  useEffect(() => {
    const extensions: Extension[] = [
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightActiveLine(),
      drawSelection(),
      history(),
      search({ top: true }),
      highlightSelectionMatches(),
      highlighter(),
      theme,
      editable.current.of(EditorView.editable.of(!readOnly)),
      Prec.high(
        keymap.of([
          { key: "Mod-s", run: () => (handlers.current.onSave(), true) },
          { key: "Mod-/", run: toggleLineComments },
        ]),
      ),
      keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) handlers.current.onChange(update.state.doc.toString());
      }),
    ];
    const editor = new EditorView({ doc: value, extensions, parent: host.current! });
    view.current = editor;
    return () => {
      editor.destroy();
      view.current = null;
    };
    // The editor is created once; `value` and `readOnly` are synced below.
  }, []);

  useEffect(() => {
    view.current?.dispatch({ effects: editable.current.reconfigure(EditorView.editable.of(!readOnly)) });
  }, [readOnly]);

  useEffect(() => {
    const editor = view.current;
    if (!editor || editor.state.doc.toString() === value) return;
    const head = Math.min(editor.state.selection.main.head, value.length);
    editor.dispatch({
      changes: { from: 0, to: editor.state.doc.length, insert: value },
      selection: EditorSelection.cursor(head),
      // Undo steps back through the user's own typing, never into text a
      // reload or write replaced.
      annotations: Transaction.addToHistory.of(false),
    });
  }, [value]);

  const lastQuery = useRef("");

  useImperativeHandle(ref, () => ({
    find: (query) => {
      const editor = view.current;
      if (!editor || !query) return;
      // A changed query searches from where the current match starts, so
      // typing "ap" after "a" keeps the match at the cursor; the same query
      // again moves on to the next one.
      const refine = query !== lastQuery.current;
      lastQuery.current = query;
      editor.dispatch({
        effects: setSearchQuery.of(new SearchQuery({ search: query, caseSensitive: false })),
        ...(refine && { selection: EditorSelection.cursor(editor.state.selection.main.from) }),
      });
      findNext(editor);
    },
    revealLine: (target) => {
      const editor = view.current;
      if (!editor) return;
      for (let n = 1; n <= editor.state.doc.lines; n++) {
        const line = editor.state.doc.line(n);
        if (line.text.trim() === target) {
          editor.dispatch({ selection: EditorSelection.cursor(line.from), scrollIntoView: true });
          editor.focus();
          return;
        }
      }
    },
  }));

  return <div ref={host} data-testid={testId} className="min-h-0 flex-1 overflow-hidden" style={{ fontSize }} />;
}
