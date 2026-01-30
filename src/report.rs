use std::fs;
use std::path::Path;

use anyhow::Result;

/// Render a simple HTML view for a benchmark JSON report produced by
/// `run_benchmark_and_save`. The JSON is embedded in a <pre> block with
/// minimal HTML-escaping so the file can be opened in a browser.
pub fn render_json_to_html(input_json: &Path, out_html: &Path) -> Result<()> {
    let json = fs::read_to_string(input_json)?;

    // Minimal HTML-escape for safety
    let escaped = json.replace('&', "&").replace('<', "<").replace('>', ">");

    let html = format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>Benchmark Report</title>\n<style>body {{ font-family: system-ui, -apple-system, Roboto, 'Segoe UI', Helvetica, Arial; padding: 1rem; }} pre {{ background:#f6f8fa; padding:1rem; border-radius:6px; overflow:auto; }}</style>\n</head>\n<body>\n<h1>Benchmark Report</h1>\n<pre>{}</pre>\n</body>\n</html>",
        escaped
    );

    if let Some(parent) = out_html.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_html, html)?;
    Ok(())
}

/// Generate a report from a saved JSON report file. This matches the
/// library-level export expected by the CLI: `generate_report(input, format, include, top_n)`.
/// For `format == "html"` a sidecar HTML file is written next to the input JSON.
/// For `format == "json"` we print an enriched JSON that includes a `total_files` key.
/// For other formats we simply print the JSON (placeholder simple behavior).
pub fn generate_report(
    input: &str,
    format: &str,
    _include: &Vec<String>,
    _top_n: usize,
) -> Result<()> {
    let in_path = Path::new(input);
    if !in_path.exists() {
        anyhow::bail!("input report not found: {}", input);
    }

    match format.to_lowercase().as_str() {
        "html" => {
            let out = in_path.with_extension("html");
            render_json_to_html(in_path, &out)?;
            println!("Wrote report HTML to {}", out.display());
            Ok(())
        }
        "json" => {
            // Read and parse the input JSON, enrich with total_files if entries present
            let s = fs::read_to_string(in_path)?;
            let mut v: serde_json::Value = serde_json::from_str(&s)?;
            let total = v
                .get("entries")
                .and_then(|e| e.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            if let Some(obj) = v.as_object_mut() {
                obj.insert(
                    "total_files".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(total)),
                );
            }
            let pretty = serde_json::to_string_pretty(&v)?;
            println!("{}", pretty);
            Ok(())
        }
        // For now treat other formats as identity: print the JSON
        _ => {
            let s = fs::read_to_string(in_path)?;
            println!("{}", s);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn render_bench_json_to_html_roundtrip() {
        let tmp = env::temp_dir();
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let in_path = tmp.join(format!("bench-{}.json", ts));
        let out_path = tmp.join(format!("bench-{}.html", ts));

        let sample = r#"{"algorithm":"testalg","size_mb":1,"time_s":0.123456,"throughput_mb_s":8.10,"timestamp_unix":1234567890}"#;
        fs::write(&in_path, sample).expect("write sample json");

        render_json_to_html(&in_path, &out_path).expect("render html");

        let html = fs::read_to_string(&out_path).expect("read html");
        assert!(html.contains("<pre>"));
        assert!(html.contains("testalg"));

        // cleanup (best effort)
        let _ = fs::remove_file(in_path);
        let _ = fs::remove_file(out_path);
    }
}
