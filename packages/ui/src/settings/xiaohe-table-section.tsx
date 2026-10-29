import { useEffect, useRef, useState } from "react";
import type { DictionaryClient } from "../dictionary/dictionary-contracts";
import { GroupList } from "../core/platform-controls";
import { ActionButton } from "./action-button";
import { SettingsGroupNote } from "./settings-group-note";
import { ErrorAlert } from "../core/error-alert";

/** 独立音形表导入；不会把条目混进拼音词库或快捷短语。 */
export function XiaoheTableSection({
  dictionary,
  onReady,
}: {
  dictionary?: DictionaryClient;
  onReady?: (ready: boolean) => void;
}) {
  const input = useRef<HTMLInputElement>(null);
  const [count, setCount] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const request = dictionary?.xiaohe;
  useEffect(() => {
    let active = true;
    request?.()
      .then((status) => {
        if (active) {
          setCount(status.count);
          onReady?.(status.installed);
        }
      })
      .catch(() => {
        if (active) {
          onReady?.(false);
          setError("无法读取小鹤音形码表，请重新导入。");
        }
      });
    return () => {
      active = false;
    };
  }, [request, onReady]);
  if (!request) return null;
  const importFile = async (file?: File) => {
    if (!file || busy) return;
    setBusy(true);
    setError("");
    try {
      if (file.size > 32 * 1024 * 1024) throw new Error("too large");
      const text = new TextDecoder("utf-8", { fatal: true }).decode(await file.arrayBuffer());
      const result = await request(text);
      setCount(result.count);
      onReady?.(result.installed);
    } catch {
      setError("导入失败。请选择小鹤音形专用 UTF-8 码表，文件不超过 32 MB；原有音形码表已保留。");
    } finally {
      setBusy(false);
      if (input.current) input.current.value = "";
    }
  };
  return (
    <GroupList title="小鹤音形码表">
      <SettingsGroupNote>
        {count === null
          ? "正在读取码表…"
          : count > 0
            ? `已导入 ${count.toLocaleString()} 条候选，可在输入方案中选择“小鹤音形”。`
            : "请先导入音形专用码表，再选择“小鹤音形”输入方案。"}
      </SettingsGroupNote>
      <SettingsGroupNote>
        按固定的一至四码选词，四码唯一候选自动上屏，分号选次选，每页五项。
      </SettingsGroupNote>
      <input
        ref={input}
        type="file"
        accept=".txt,.tsv"
        hidden
        onChange={(event) => {
          void importFile(event.target.files?.[0]);
        }}
      />
      <ActionButton
        disabled={busy}
        action={() => input.current?.click()}
        label={busy ? "正在导入…" : "导入音形码表…"}
      />
      {error && <ErrorAlert>{error}</ErrorAlert>}
    </GroupList>
  );
}
