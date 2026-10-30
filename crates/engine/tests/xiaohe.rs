use msime_engine::xiaohe::XiaoheTable;
use rusqlite::Connection;

#[test]
fn xiaohe_has_its_own_scheme_and_fixed_order_traits() {
    let scheme = msime_engine::SchemeType::from_u8(10).expect("independent scheme");
    assert_eq!(scheme.name(), "xiaohe_yinxing");
    assert!(scheme.is_chinese());
    assert!(scheme.selection_completes());
    assert!(!scheme.is_pinyin());
    assert!(!scheme.learns_into_main_dictionary());
    assert!(!scheme.allows_english_emoji_mixing());
    assert!(!scheme.reranks_with_sentence_model());
}

#[test]
fn fixed_codes_keep_reference_order_and_require_complete_code() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("table.db");
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "PRAGMA user_version=1; CREATE TABLE entries(code TEXT, rank INTEGER, text TEXT);
         INSERT INTO entries VALUES('ab',2,'乙'),('ab',1,'甲'),('abc',1,'丙'),('z',1,'丁');",
        )
        .unwrap();
    let table = XiaoheTable::open(&path).unwrap();
    assert_eq!(
        table
            .query("ab")
            .unwrap()
            .iter()
            .map(|x| x.word.as_str())
            .collect::<Vec<_>>(),
        ["甲", "乙"]
    );
    assert!(table.query("a").unwrap().is_empty());
    assert_eq!(table.query("z").unwrap()[0].word, "丁");
    assert!(table.query("abcde").unwrap().is_empty());
}

#[test]
fn personal_xiaohe_words_follow_the_base_table_and_survive_reimport() {
    let root = tempfile::tempdir().unwrap();
    msime_engine::xiaohe::import_tsv(root.path(), "# msime-xiaohe-yinxing-v1\na\t1\t甲\n").unwrap();
    let table = XiaoheTable::open(&root.path().join("xiaohe-yinxing.db")).unwrap();
    msime_engine::xiaohe::edit_user_entry(root.path(), "a", "乙", true).unwrap();
    assert_eq!(
        table
            .query("a")
            .unwrap()
            .iter()
            .map(|row| row.word.as_str())
            .collect::<Vec<_>>(),
        ["甲", "乙"]
    );
    drop(table);
    assert_eq!(
        msime_engine::xiaohe::list_user_entries(root.path()).unwrap(),
        [("a".into(), "乙".into())]
    );
    msime_engine::xiaohe::import_tsv(root.path(), "# msime-xiaohe-yinxing-v1\na\t1\t丙\n").unwrap();
    let table = XiaoheTable::open(&root.path().join("xiaohe-yinxing.db")).unwrap();
    assert_eq!(
        table
            .query("a")
            .unwrap()
            .iter()
            .map(|row| row.word.as_str())
            .collect::<Vec<_>>(),
        ["丙", "乙"]
    );
    drop(table);
    msime_engine::xiaohe::edit_user_entry(root.path(), "a", "乙", false).unwrap();
    assert!(msime_engine::xiaohe::list_user_entries(root.path())
        .unwrap()
        .is_empty());
}

#[test]
fn missing_and_unknown_version_tables_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("table.db");
    assert!(XiaoheTable::open(&path).is_err());
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "PRAGMA user_version=2; CREATE TABLE entries(code TEXT,rank INTEGER,text TEXT)",
        )
        .unwrap();
    assert!(XiaoheTable::open(&path).is_err());
}

fn session() -> (tempfile::TempDir, msime_engine::Session) {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("helpcodes")).unwrap();
    std::fs::write(root.path().join("helpcodes/helpcode.txt"), "甲=ab\n").unwrap();
    std::fs::write(
        root.path().join("helpcodes/xiaohe_helpcode.txt"),
        "甲=xy\n乙=uv\n丙=rs\n",
    )
    .unwrap();
    Connection::open(root.path().join("xiaohe-yinxing.db"))
        .unwrap()
        .execute_batch(
            "PRAGMA user_version=1; CREATE TABLE entries(code TEXT,rank INTEGER,text TEXT);
         INSERT INTO entries VALUES('a',1,'甲'),('ab',1,'乙'),('ab',2,'丙'),
         ('abcd',1,'丁'),('abcd',2,'戊'),('xyzz',1,'己'),(';q',1,'！');",
        )
        .unwrap();
    let paths = msime_engine::RuntimePaths {
        resources: root.path().into(),
        user_data: root.path().into(),
        cache: root.path().into(),
        dictionaries: root.path().into(),
    };
    let mut options = msime_engine::SessionOptions::new(paths);
    options.scheme = msime_engine::SchemeType::XiaoheYinxing;
    options.enabled_schemes = msime_engine::SchemeSet::of(&[options.scheme]);
    let session = msime_engine::Session::new(options).unwrap();
    (root, session)
}

#[test]
fn xiaohe_candidates_show_xiaohe_shape_hints_without_changing_fixed_order() {
    let (_root, mut session) = session();
    session.character(b'a', false);
    let first = session.snapshot();
    assert_eq!(first.candidates[0].word, "甲");
    assert_eq!(first.candidate_annotations[0], "(xY)");

    session.character(b'b', false);
    let second = session.snapshot();
    assert_eq!(
        second
            .candidates
            .iter()
            .map(|candidate| candidate.word.as_str())
            .collect::<Vec<_>>(),
        ["乙", "丙"]
    );
    assert_eq!(second.candidate_annotations, ["(uV)", "(rS)"]);
    assert_eq!(session.character(b';', false).commit.as_deref(), Some("丙"));
}

#[test]
fn xiaohe_shape_hints_do_not_borrow_a_changed_pinyin_helpcode_table() {
    let (_root, mut session) = session();
    let changed =
        msime_engine::host::HelpcodeKeymap::from_codes([("甲".to_owned(), "zz".to_owned())].into());
    session.set_helpcode_table(std::sync::Arc::new(changed));
    session.character(b'a', false);
    assert_eq!(session.snapshot().candidate_annotations[0], "(xY)");
}

#[test]
fn xiaohe_missing_shape_hint_resource_keeps_table_input_working() {
    let (root, _old) = session();
    std::fs::remove_file(root.path().join("helpcodes/xiaohe_helpcode.txt")).unwrap();
    let paths = msime_engine::RuntimePaths {
        resources: root.path().into(),
        user_data: root.path().into(),
        cache: root.path().into(),
        dictionaries: root.path().into(),
    };
    let mut options = msime_engine::SessionOptions::new(paths);
    options.scheme = msime_engine::SchemeType::XiaoheYinxing;
    options.enabled_schemes = msime_engine::SchemeSet::of(&[options.scheme]);
    let mut fresh = msime_engine::Session::new(options).unwrap();
    fresh.character(b'a', false);
    assert_eq!(fresh.snapshot().candidates[0].word, "甲");
    assert_eq!(fresh.snapshot().candidate_annotations[0], "");
}

#[test]
fn xiaohe_cloud_answer_appends_after_fixed_rows_and_rejects_old_input() {
    let (_root, mut session) = session();
    session.set_cloud_candidates_enabled(true);
    session.character(b'a', false);
    let query = session.online_query().expect("xiaohe query");
    assert_eq!(query.query_text, "a");
    assert!(query.pinyin_segments.is_empty());
    assert!(query.cloud_eligible);
    assert!(!query.ai_eligible);
    assert!(session.apply_online_candidates(
        &query,
        &["云甲".into()],
        msime_engine::CandidateSource::CloudSuggestion,
    ));
    let shown = session.snapshot();
    assert_eq!(shown.candidates[0].word, "甲");
    assert_eq!(shown.candidates[1].word, "云甲");
    assert_eq!(shown.candidate_annotations[1], "");
    session.character(b'b', false);
    assert_eq!(session.snapshot().candidates[0].word, "乙");
    assert!(!session.apply_online_candidates(
        &query,
        &["旧答案".into()],
        msime_engine::CandidateSource::CloudSuggestion,
    ));
}

#[test]
fn xiaohe_selects_second_and_consumes_the_whole_code() {
    let (_root, mut s) = session();
    for c in b"ab" {
        assert!(s.character(*c, false).handled);
    }
    assert_eq!(s.snapshot().candidates[0].word, "乙");
    assert_eq!(s.character(b';', false).commit.as_deref(), Some("丙"));
    assert!(s.snapshot().preedit.is_empty());
}

#[test]
fn xiaohe_unique_four_code_commits_and_fifth_key_keeps_its_new_code() {
    let (_root, mut s) = session();
    for c in b"xyz" {
        assert!(s.character(*c, false).commit.is_none());
    }
    assert_eq!(s.character(b'z', false).commit.as_deref(), Some("己"));
    for c in b"abcd" {
        assert!(s.character(*c, false).commit.is_none());
    }
    assert_eq!(s.character(b'a', false).commit.as_deref(), Some("丁"));
    assert_eq!(s.snapshot().preedit, "a");
    assert_eq!(s.snapshot().candidates[0].word, "甲");
}

#[test]
fn xiaohe_import_replaces_atomically_and_keeps_old_table_on_failure() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        msime_engine::xiaohe::import_tsv(
            root.path(),
            "# msime-xiaohe-yinxing-v1\nab\t1\t甲\nab\t2\t乙\n"
        )
        .unwrap(),
        2
    );
    assert!(msime_engine::xiaohe::import_tsv(
        root.path(),
        "# msime-xiaohe-yinxing-v1\nab\t1\t丙\nab\t1\t丁\n"
    )
    .is_err());
    let table = XiaoheTable::open(&root.path().join("xiaohe-yinxing.db")).unwrap();
    assert_eq!(table.query("ab").unwrap()[0].word, "甲");
}

#[test]
fn xiaohe_empty_code_clears_on_next_key_and_tab_cancels() {
    let (_root, mut s) = session();
    for c in b"zzzz" {
        assert!(s.character(*c, false).commit.is_none());
    }
    assert_eq!(s.character(b'a', false).commit, None);
    assert_eq!(s.snapshot().preedit, "a");
    assert!(s.snapshot().spelling_symbols.contains('\t'));
    assert!(s.character(b'\t', false).handled);
    assert!(s.snapshot().preedit.is_empty());
    assert!(s.character(b';', false).handled);
    assert_eq!(s.character(b'q', false).commit.as_deref(), Some("！"));
}

#[test]
fn xiaohe_apostrophe_after_four_code_matches_rime_final_key() {
    let (_root, mut s) = session();
    for c in b"abcd" {
        s.character(*c, false);
    }
    assert!(s.character(b'\'', false).commit.is_none());
    assert_eq!(s.snapshot().preedit, "abcd'");
    assert!(s.snapshot().candidates.is_empty());
    assert!(s.character(b'a', false).commit.is_none());
    assert_eq!(s.snapshot().preedit, "a");
}
