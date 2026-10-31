#!/usr/bin/env python3
"""为个人定制包生成独立音形基线；不读取偏好、账号或历史记录。"""
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import sys
import tempfile


def stage(source: Path, resources: Path) -> None:
    if source.is_symlink() or not source.is_file() or source.stat().st_size > 32 * 1024 * 1024:
        raise ValueError("音形源文件无效")
    with sqlite3.connect(source.resolve().as_uri() + "?mode=ro", uri=True) as db:
        if db.execute("PRAGMA user_version").fetchone()[0] != 1:
            raise ValueError("音形源版本无效")
        rows = db.execute("SELECT code,rank,text FROM entries ORDER BY code,rank").fetchall()
    if not rows or len(rows) > 500_000:
        raise ValueError("音形条数无效")
    for code, rank, word in rows:
        letters = "abcdefghijklmnopqrstuvwxyz"
        valid_code = isinstance(code, str) and (
            (1 <= len(code) <= 4 and all(c in letters for c in code))
            or code == ";" or (len(code) == 2 and code[0] == ";" and code[1] in letters))
        if (not valid_code
                or not isinstance(rank, int) or rank < 1 or not isinstance(word, str) or not word
                or len(word.encode("utf-16-le")) > 398
                or any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in word)):
            raise ValueError("音形条目无效")
    seed = resources / "xiaohe-yinxing"
    seed.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=".seed-", dir=seed)
    os.close(fd)
    try:
        with sqlite3.connect(name) as out:
            # 重建白名单表，避免复制源库的历史页、附加表或本机信息。
            out.executescript("PRAGMA user_version=1; CREATE TABLE entries(code TEXT NOT NULL,rank INTEGER NOT NULL,text TEXT NOT NULL,UNIQUE(code,text),UNIQUE(code,rank)); CREATE INDEX entries_code_rank ON entries(code,rank);")
            out.executemany("INSERT INTO entries VALUES(?,?,?)", rows)
        payload = Path(name).read_bytes()
        if len(payload) > 32 * 1024 * 1024:
            raise ValueError("生成的音形表过大")
        manifest = {"format_version": 1, "sha256": hashlib.sha256(payload).hexdigest(),
                    "size": len(payload), "entries": len(rows), "default_scheme": "xiaohe_yinxing"}
        staged_db = seed / "xiaohe-yinxing.db"
        os.replace(name, staged_db)
        # 系统安装会把文件归 root 所有；输入法以普通用户身份读取这份公开码表。
        os.chmod(staged_db, 0o644)
        (seed / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
        (seed / "使用说明.txt").write_text("个人小鹤音形定制码表。首次安装默认小鹤音形；重装仅补缺表，不覆盖用户已有码表和偏好。码表来自用户提供及授权整合的本地数据，未包含账号、密钥或历史输入。\n")
    finally:
        Path(name).unlink(missing_ok=True)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("用法：stage_xiaohe_seed.py 源码表.db 目标Resources目录")
    stage(Path(sys.argv[1]), Path(sys.argv[2]))
