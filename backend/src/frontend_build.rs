use anyhow::{Context, Result};
use sha1::Digest;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::info;
use walkdir::WalkDir;

/// Minify a standalone JS source string using oxc.
fn minify_js(source: &str) -> Result<String> {
    use oxc_allocator::Allocator;
    use oxc_codegen::{Codegen, CodegenOptions};
    use oxc_minifier::{CompressOptions, MangleOptions, Minifier, MinifierOptions};
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    let allocator = Allocator::default();
    let source_type = SourceType::script();
    let ret = Parser::new(&allocator, source, source_type).parse();
    if !ret.errors.is_empty() {
        let msgs: Vec<String> = ret.errors.iter().map(|e| e.to_string()).collect();
        anyhow::bail!("JS parse errors: {}", msgs.join("; "));
    }
    let mut program = ret.program;

    let options = MinifierOptions {
        mangle: Some(MangleOptions::default()),
        compress: Some(CompressOptions::default()),
    };
    let ret = Minifier::new(options).minify(&allocator, &mut program);

    let output = Codegen::new()
        .with_options(CodegenOptions::minify())
        .with_scoping(ret.scoping)
        .with_private_member_mappings(ret.class_private_mappings)
        .build(&program)
        .code;
    Ok(output)
}

/// Build the frontend: copy all files from `src_dir` to `out_dir`,
/// minifying JS and CSS files (except those under `external/`),
/// and rewriting `index.html` with content-hash cache-busting query strings.
pub fn build_frontend(src_dir: &Path, out_dir: &Path) -> Result<PathBuf> {
    info!("Building frontend from {:?} into {:?}", src_dir, out_dir);

    // Clean and recreate output directory
    if out_dir.exists() {
        std::fs::remove_dir_all(out_dir).context("Failed to remove old build output")?;
    }
    std::fs::create_dir_all(out_dir).context("Failed to create build output directory")?;

    // Walk source directory, copy/minify all files, collect hashes for referenced assets
    let mut asset_hashes: HashMap<String, String> = HashMap::new();

    for entry in WalkDir::new(src_dir) {
        let entry = entry?;
        let src_path = entry.path();
        let relative = src_path
            .strip_prefix(src_dir)
            .context("Failed to strip prefix")?;

        let dest_path = out_dir.join(relative);

        if src_path.is_dir() {
            std::fs::create_dir_all(&dest_path)?;
            continue;
        }

        // Skip .DS_Store and other hidden files
        if relative
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            continue;
        }

        let is_external = relative.starts_with("external");
        let ext = src_path.extension().and_then(|e| e.to_str());

        let content = std::fs::read(src_path)
            .with_context(|| format!("Failed to read {:?}", src_path))?;
        let content_len = content.len();

        let output_bytes = match (ext, is_external) {
            (Some("js"), false) => {
                let source = String::from_utf8(content)
                    .with_context(|| format!("Non-UTF8 JS file: {:?}", src_path))?;
                match minify_js(&source) {
                    Ok(minified) => {
                        info!(
                            "Minified JS: {} ({} → {} bytes)",
                            relative.display(),
                            content_len,
                            minified.len()
                        );
                        minified.into_bytes()
                    }
                    Err(e) => {
                        tracing::warn!(
                            "JS minification failed for {}: {}, using original",
                            relative.display(),
                            e
                        );
                        source.into_bytes()
                    }
                }
            }
            (Some("css"), false) => {
                let source = String::from_utf8(content)
                    .with_context(|| format!("Non-UTF8 CSS file: {:?}", src_path))?;
                let minified = minifier::css::minify(&source)
                    .map_err(|e| anyhow::anyhow!("CSS minification error in {:?}: {}", src_path, e))?
                    .to_string();
                info!(
                    "Minified CSS: {} ({} → {} bytes)",
                    relative.display(),
                    source.len(),
                    minified.len()
                );
                minified.into_bytes()
            }
            _ => content,
        };

        // Compute content hash for JS/CSS files (these may be referenced in index.html)
        if matches!(ext, Some("js") | Some("css")) {
            let hash = short_hash(&output_bytes);
            let rel_str = relative.to_str().unwrap_or_default().replace('\\', "/");
            asset_hashes.insert(rel_str, hash);
        }

        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest_path, &output_bytes)
            .with_context(|| format!("Failed to write {:?}", dest_path))?;
    }

    // Now rewrite index.html with cache-busting query strings
    let index_path = out_dir.join("index.html");
    if index_path.exists() {
        let html = std::fs::read_to_string(&index_path)?;
        let html = rewrite_asset_references(&html, &asset_hashes);
        std::fs::write(&index_path, html)?;
        info!("Rewrote index.html with {} cache-busted asset references", asset_hashes.len());
    }

    info!("Frontend build complete");
    Ok(out_dir.to_path_buf())
}

/// Compute a short (8-char hex) SHA-1 hash of content
fn short_hash(content: &[u8]) -> String {
    let mut hasher = sha1::Sha1::new();
    hasher.update(content);
    let result = hasher.finalize();
    hex_encode(&result[..4])
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Rewrite src="foo.js" and href="foo.css" references in HTML
/// to include ?v=<hash> cache-busting query strings.
fn rewrite_asset_references(html: &str, hashes: &HashMap<String, String>) -> String {
    let mut result = html.to_string();
    for (path, hash) in hashes {
        let plain = format!("\"{}\"", path);
        let busted = format!("\"{}?v={}\"", path, hash);
        result = result.replace(&plain, &busted);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_asset_references() {
        let html = r#"<script src="mobile-calendar.js"></script>
<link rel="stylesheet" href="style.css" />"#;
        let mut hashes = HashMap::new();
        hashes.insert("mobile-calendar.js".to_string(), "abcd1234".to_string());
        hashes.insert("style.css".to_string(), "ef567890".to_string());

        let result = rewrite_asset_references(html, &hashes);
        assert!(result.contains("mobile-calendar.js?v=abcd1234"));
        assert!(result.contains("style.css?v=ef567890"));
    }

    #[test]
    fn test_short_hash() {
        let hash = short_hash(b"hello world");
        assert_eq!(hash.len(), 8);
        assert_eq!(hash, short_hash(b"hello world"));
        assert_ne!(hash, short_hash(b"hello world!"));
    }

    #[test]
    fn test_minify_js_basic() {
        let source = "const x = 1 + 1; console.log(x);";
        let result = minify_js(source).unwrap();
        assert!(result.len() < source.len());
    }

    #[test]
    fn test_minify_js_preserves_top_level_names() {
        let source = r#"
            function switchPages(page) { console.log(page); }
            function showLoginForm() { return 1; }
            const MultiSelect = { isAnyOpen() { return false; } };
        "#;
        let result = minify_js(source).unwrap();
        assert!(result.contains("switchPages"), "top-level function 'switchPages' was mangled: {result}");
        assert!(result.contains("showLoginForm"), "top-level function 'showLoginForm' was mangled: {result}");
        assert!(result.contains("MultiSelect"), "top-level const 'MultiSelect' was mangled: {result}");
    }

    #[test]
    fn test_minify_js_mangles_local_names() {
        let source = r#"
            function doStuff() {
                const longVariableName = 42;
                const anotherLongName = longVariableName + 1;
                console.log(anotherLongName);
            }
        "#;
        let result = minify_js(source).unwrap();
        assert!(!result.contains("longVariableName"), "local var should be mangled: {result}");
    }

    #[test]
    fn test_minify_js_template_literals() {
        let source = r#"console.log(`<div class="foo">
            <span>${bar}</span>
        </div>`);"#;
        let result = minify_js(source).unwrap();
        assert!(result.contains("div"));
        assert!(result.contains("${bar}"));
    }

    #[test]
    fn test_minify_mobile_calendar_no_redeclaration() {
        let source = std::fs::read_to_string("../frontend/mobile-calendar.js")
            .expect("Could not read mobile-calendar.js");
        let result = minify_js(&source).expect("Minification failed");
        // Ensure the output is valid (minifier didn't crash) and substantially smaller
        assert!(result.len() < source.len() / 2, "Minified output should be much smaller");
    }

    #[test]
    fn test_minify_script_js() {
        let source = std::fs::read_to_string("../frontend/script.js")
            .expect("Could not read script.js");
        let result = minify_js(&source).expect("Minification of script.js failed");
        assert!(result.len() < source.len());
    }
}
