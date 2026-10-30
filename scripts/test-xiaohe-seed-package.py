#!/usr/bin/env python3
"""定制基线必须保留候选顺序，且只带白名单表。"""
import hashlib
import importlib.util
import json
from pathlib import Path
import sqlite3
import tempfile

script = Path(__file__).resolve().parents[1] / "platforms/macos/scripts/stage_xiaohe_seed.py"
spec = importlib.util.spec_from_file_location("stage_xiaohe_seed", script)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
with tempfile.TemporaryDirectory() as temp:
    root = Path(temp)
    source = root / "user.db"
    with sqlite3.connect(source) as db:
        db.executescript("PRAGMA user_version=1; CREATE TABLE entries(code TEXT,rank INTEGER,text TEXT); CREATE TABLE private_history(secret TEXT); INSERT INTO private_history VALUES('synthetic-private-token');")
        db.executemany("INSERT INTO entries VALUES(?,?,?)", [("zz", 2, "例二"), ("zz", 1, "例一"), (";a", 1, "符号")])
    module.stage(source, root / "Resources")
    seed = root / "Resources/xiaohe-yinxing"
    payload = (seed / "xiaohe-yinxing.db").read_bytes()
    manifest = json.loads((seed / "manifest.json").read_text())
    assert manifest["sha256"] == hashlib.sha256(payload).hexdigest()
    assert manifest["entries"] == 3 and manifest["size"] == len(payload)
    assert b"synthetic-private-token" not in payload
    with sqlite3.connect(seed / "xiaohe-yinxing.db") as db:
        assert db.execute("SELECT code,rank,text FROM entries ORDER BY code,rank").fetchall() == [(";a", 1, "符号"), ("zz", 1, "例一"), ("zz", 2, "例二")]
        assert db.execute("SELECT name FROM sqlite_master WHERE type='table'").fetchall() == [("entries",)]
    with sqlite3.connect(source) as db:
        db.execute("INSERT INTO entries VALUES('BAD', 1, '错误')")
    try:
        module.stage(source, root / "Resources")
        raise AssertionError("无效源表未被拒绝")
    except ValueError:
        pass
    assert (seed / "xiaohe-yinxing.db").read_bytes() == payload
print("PASS: 音形基线保序、指纹、隐私表剔除及坏源回滚")
