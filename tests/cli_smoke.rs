use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs::{copy, create_dir_all, write};
use std::process::Command;
use tempfile::tempdir;

#[test]
fn cli_smoke_suite() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    create_dir_all(&src)?;
    create_dir_all(&dst)?;

    // create small files
    write(src.join("a.txt"), b"hello")?;
    write(src.join("b.txt"), b"world")?;
    // create a modified copy in dst so compare finds differences
    copy(src.join("a.txt"), dst.join("a.txt"))?;
    write(dst.join("b.txt"), b"changed")?;

    // 1) hashmap -> generate map1 and map2
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

    // 2) compare maps
    Command::new(assert_cmd::cargo::cargo_bin!("hash-folderoo"))
        .args([
            "compare",
            "--source",
            map1.to_str().unwrap(),
            "--target",
            map2.to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"changed\"").or(predicate::str::contains("changed")));

    // 3) copydiff dry-run plan generation
    Command::new(assert_cmd::cargo::cargo_bin!("hash-folderoo"))
        .args([
            "copydiff",
            "--source",
            map1.to_str().unwrap(),
            "--target",
            map2.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success();

    // 4) removempty dry-run (should succeed on existing dir)
    Command::new(assert_cmd::cargo::cargo_bin!("hash-folderoo"))
        .args([
            "removempty",
            "--path",
            dir.path().to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success();

    // 5) renamer dry-run (pattern is arbitrary for smoke test)
    Command::new(assert_cmd::cargo::cargo_bin!("hash-folderoo"))
        .args([
            "renamer",
            "--path",
            dir.path().to_str().unwrap(),
            "--pattern",
            "x",
            "--dry-run",
        ])
        .assert()
        .success();

    // 6) report on generated map (json)
    Command::new(assert_cmd::cargo::cargo_bin!("hash-folderoo"))
        .args([
            "report",
            "--input",
            map1.to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_files\"").or(predicate::str::contains("Total files")));

    // 7) benchmark basic run (small buffer)
    Command::new(assert_cmd::cargo::cargo_bin!("hash-folderoo"))
        .args(["benchmark", "--algorithm", "blake3", "--size", "1024"])
        .assert()
        .success();

    Ok(())
}
