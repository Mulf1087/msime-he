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
  const [code, setCode] = useState("");
  const [word, setWord] = useState("");
  const [personal, setPersonal] = useState<{ code: string; word: string }[]>([]);
  const [personalTotal, setPersonalTotal] = useState(0);
  const [personalOffset, setPersonalOffset] = useState(0);
  const request = dictionary?.xiaohe;
  const userRequest = dictionary?.xiaoheUser;
  const reloadPersonal = async (offset: number) => {
    if (!userRequest) return;
    const result = await userRequest("list", undefined, undefined, offset);
    setPersonal(result.entries ?? []);
    setPersonalTotal(result.total ?? 0);
    setPersonalOffset(offset);
  };
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
  useEffect(() => {
    if (count && userRequest)
      void reloadPersonal(0).catch(() => setError("无法读取个人音形词条。"));
    // A successful import or first status read is enough to refresh the list.
  }, [count, userRequest]);
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
      {count !== null && count > 0 && userRequest && (
        <>
          <SettingsGroupNote>
            个人词条排在原码表候选之后；通过“词库同步”可同步到另一台已安装小鹤音形码表的电脑。
          </SettingsGroupNote>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <input
              aria-label="音形编码"
              placeholder="一至四码"
              value={code}
              maxLength={4}
              onChange={(event) => setCode(event.target.value.toLowerCase())}
            />
            <input
              aria-label="个人词条"
              placeholder="词条"
              value={word}
              onChange={(event) => setWord(event.target.value)}
            />
            <ActionButton
              disabled={busy || !code || !word}
              label="添加个人词条"
              action={() => {
                setBusy(true);
                setError("");
                void userRequest("add", code, word)
                  .then(() => {
                    setCode("");
                    setWord("");
                    return reloadPersonal(0);
                  })
                  .catch(() => setError("添加失败，请检查编码和词条。"))
                  .finally(() => setBusy(false));
              }}
            />
          </div>
          <SettingsGroupNote>个人词条：{personalTotal.toLocaleString()} 条</SettingsGroupNote>
          {personal.map((entry) => (
            <div
              key={`${entry.code}\u0000${entry.word}`}
              style={{ display: "flex", gap: 8, alignItems: "center" }}
            >
              <span>
                {entry.code}　{entry.word}
              </span>
              <ActionButton
                disabled={busy}
                label="删除"
                action={() => {
                  setBusy(true);
                  setError("");
                  void userRequest("delete", entry.code, entry.word)
                    .then(() => reloadPersonal(personalOffset))
                    .catch(() => setError("删除个人词条失败。"))
                    .finally(() => setBusy(false));
                }}
              />
            </div>
          ))}
          {personalOffset > 0 && (
            <ActionButton
              label="上一页"
              action={() => void reloadPersonal(Math.max(0, personalOffset - 100))}
            />
          )}
          {personalOffset + personal.length < personalTotal && (
            <ActionButton label="下一页" action={() => void reloadPersonal(personalOffset + 100)} />
          )}
        </>
      )}
      {error && <ErrorAlert>{error}</ErrorAlert>}
    </GroupList>
  );
}
