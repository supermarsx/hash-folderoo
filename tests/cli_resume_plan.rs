use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs::{create_dir_all, write};
use std::process::Command;
use tempfile::tempdir;

#[test]
fn copydiff_resume_executes_and_persists_plan() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    create_dir_all(&src)?;
    create_dir_all(&dst)?;

    // create a single source file
    write(src.join("a.txt"), b"hello")?;

    // create a minimal plan JSON referencing src/dst
    let plan = dir.path().join("plan.json");
    let plan_json = serde_json::json!({
        "meta": { "version": 1, "generated_at": "now" },
        "ops": [
            { "src": src.join("a.txt").to_string_lossy(), "dst": dst.join("a.txt").to_string_lossy(), "op": "copy", "done": false }
        ]
    });
    std::fs::write(&plan, serde_json::to_string_pretty(&plan_json)?)?;

    // run the CLI with --plan + --execute + --resume
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("hash-folderoo"));
    cmd.args([
        "copydiff",
        "--plan",
        plan.to_str().unwrap(),
        "--execute",
        "--resume",
    ]);

    cmd.assert().success();

    // destination file should exist
    assert!(dst.join("a.txt").exists());

    // persisted plan should contain status/done markings
    let contents = std::fs::read_to_string(&plan)?;
    assert!(contents.contains("\"status\": \"done\"") || contents.contains("\"done\": true"));

    Ok(())
}
