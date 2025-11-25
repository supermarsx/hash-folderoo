use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs::{copy, create_dir_all, write};
use std::process::Command;
use tempfile::tempdir;

#[test]
fn copydiff_dry_run_git_diff_shows_content_hunks() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    create_dir_all(&src)?;
    create_dir_all(&dst)?;

    // create small files where b.txt differs
    write(src.join("a.txt"), b"hello")?;
    write(src.join("b.txt"), b"world")?;
    copy(src.join("a.txt"), dst.join("a.txt"))?;
    write(dst.join("b.txt"), b"changed")?;

    // maps
    let map1 = dir.path().join("map1.json");
    Command::new(assert_cmd::cargo::cargo_bin!("hash-folderoo"))
        .args([
            "hashmap",
            "--path",
            src.to_str().unwrap(),
            "--output",
            map1.to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success();

    let map2 = dir.path().join("map2.json");
    Command::new(assert_cmd::cargo::cargo_bin!("hash-folderoo"))
        .args([
            "hashmap",
            "--path",
            dst.to_str().unwrap(),
            "--output",
            map2.to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success();

    // Run copydiff dry-run with git-diff and ensure unified-like body is present
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("hash-folderoo"));
    cmd.args([
        "copydiff",
        "--source",
        map1.to_str().unwrap(),
        "--target",
        map2.to_str().unwrap(),
        "--dry-run",
        "--git-diff",
    ]);

    cmd.assert()
        .success()
        // at minimum, we expect a git-style header and a content line for the differing file (+world or unchanged ' world')
        .stdout(
            predicate::str::contains("diff --git").and(
                predicate::str::contains("+world").or(predicate::str::contains(" world")),
            ),
        );

    Ok(())
}

#[test]
fn renamer_dry_run_git_diff_shows_content_for_same_file() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let root = dir.path().join("root");
    create_dir_all(&root)?;
    write(root.join("a.txt"), b"hello world")?;

    // run renamer with simple substring mapping to cause a rename
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("hash-folderoo"));
    cmd.args([
        "renamer",
        "--path",
        root.to_str().unwrap(),
        "--pattern",
        "a->b",
        "--dry-run",
        "--git-diff",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("rename from").and(predicate::str::contains("hello world")));

    Ok(())
}
