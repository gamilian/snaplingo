import { useEffect, useState } from 'react';

interface FavoriteMetadataEditorProps {
  note?: string;
  tags?: string[];
  onSaveNote(note: string): Promise<void>;
  onSaveTags(tags: string[]): Promise<void>;
}

export function FavoriteMetadataEditor({
  note,
  tags = [],
  onSaveNote,
  onSaveTags,
}: FavoriteMetadataEditorProps) {
  const [noteDraft, setNoteDraft] = useState(note ?? '');
  const [tagDraft, setTagDraft] = useState(tags.join(', '));

  useEffect(() => setNoteDraft(note ?? ''), [note]);
  useEffect(() => setTagDraft(tags.join(', ')), [tags]);

  return (
    <div className="mt-3 grid gap-2 rounded-lg bg-sunken p-3">
      <label className="grid gap-1 text-xs text-muted">
        笔记
        <textarea
          value={noteDraft}
          onChange={(event) => setNoteDraft(event.target.value)}
          onBlur={() => void onSaveNote(noteDraft)}
          rows={2}
          className="resize-none rounded-md border border-line bg-surface px-3 py-2 text-sm text-ink outline-none focus:border-accent"
          placeholder="添加上下文笔记"
        />
      </label>
      <label className="grid gap-1 text-xs text-muted">
        标签（使用逗号分隔）
        <input
          value={tagDraft}
          onChange={(event) => setTagDraft(event.target.value)}
          onBlur={() =>
            void onSaveTags(
              tagDraft
                .split(',')
                .map((tag) => tag.trim())
                .filter(Boolean),
            )
          }
          className="rounded-md border border-line bg-surface px-3 py-2 text-sm text-ink outline-none focus:border-accent"
          placeholder="工作, 技术"
        />
      </label>
    </div>
  );
}
