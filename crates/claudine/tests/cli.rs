use assert_cmd::Command;
use predicates::str::contains;

fn fake_home_with_session(base: &std::path::Path) {
    let pdir = base.join("projects").join("-home-old-proj");
    std::fs::create_dir_all(&pdir).unwrap();
    std::fs::write(
        pdir.join("abc.jsonl"),
        r#"{"cwd":"/home/old/proj","timestamp":"t"}"#,
    )
    .unwrap();
}

#[test]
fn bare_invocation_launches_tui() {
    // Sans TTY (environnement de test/CI), la TUI ne peut pas passer en raw mode :
    // elle doit échouer proprement (code non nul + message sur stderr), sans
    // jamais laisser le terminal cassé. On ne peut pas piloter la TUI ici.
    Command::cargo_bin("claudine")
        .unwrap()
        .write_stdin("")
        .assert()
        .failure()
        .stderr(contains("Erreur"));
}

#[test]
fn export_then_import_dry_run_roundtrip() {
    let src = tempfile::tempdir().unwrap();
    fake_home_with_session(src.path());
    let bundle = src.path().join("bundle.tar.gz");

    // export
    Command::cargo_bin("claudine")
        .unwrap()
        .env("CLAUDE_CONFIG_DIR", src.path())
        .args(["export", "--out", bundle.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Bundle écrit"));
    assert!(bundle.exists());

    // import dry-run dans une nouvelle home
    let dst = tempfile::tempdir().unwrap();
    Command::cargo_bin("claudine")
        .unwrap()
        .env("CLAUDE_CONFIG_DIR", dst.path())
        .args([
            "import",
            bundle.to_str().unwrap(),
            "--map",
            "/home/old=/home/new",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(contains("sessions_new: 1"))
        .stdout(contains("dry-run"));

    // la cible n'a rien reçu
    assert!(!dst.path().join("projects/-home-new-proj").exists());
}

#[test]
fn prints_version() {
    Command::cargo_bin("claudine")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("claudine 0.1.1"));
}

#[test]
fn help_lists_subcommands() {
    Command::cargo_bin("claudine")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("export"))
        .stdout(contains("import"))
        .stdout(contains("homes"));
}

#[test]
fn unknown_flag_fails() {
    Command::cargo_bin("claudine")
        .unwrap()
        .arg("--nope")
        .assert()
        .failure();
}
