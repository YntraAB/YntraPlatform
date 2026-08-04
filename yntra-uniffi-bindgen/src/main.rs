use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Preserve 100% compatibility with standard UniFFI commands if running "generate"
    if args.len() > 1 && args[1] == "generate" {
        uniffi::uniffi_bindgen_main();
        return;
    }

    let mut is_watch = false;
    let mut is_release = false;
    let mut is_install_hooks = false;
    let mut is_check_locales = false;
    let mut is_check_parity = false;
    let mut is_fix = false;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "watch" | "--watch" => is_watch = true,
            "release" | "--release" => is_release = true,
            "install-hooks" | "--install-hooks" | "install_hooks" => is_install_hooks = true,
            "check-locales" | "--check-locales" | "check_locales" => is_check_locales = true,
            "check-parity" | "--check-parity" | "check_parity" => is_check_parity = true,
            "fix" | "--fix" => is_fix = true,
            _ => {}
        }
    }

    let workspace_root = match find_workspace_root() {
        Some(root) => root,
        None => {
            eprintln!(
                "Error: Could not find workspace root (containing Cargo.toml with [workspace])."
            );
            std::process::exit(1);
        }
    };

    if is_install_hooks {
        if let Err(e) = install_hooks(&workspace_root) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        println!("Successfully installed git pre-commit hook!");
        return;
    }

    if is_check_parity {
        match run_check_parity(&workspace_root) {
            Ok(true) => {
                println!("All UniFFI exported APIs are clean and synchronized!");
                return;
            }
            Ok(false) => {
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Error checking UniFFI API parity: {}", e);
                std::process::exit(1);
            }
        }
    }

    if is_check_locales {
        match run_check_locales(&workspace_root, is_fix) {
            Ok(true) => {
                println!("All localization catalogs are clean and consistent!");
                return;
            }
            Ok(false) => {
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Error checking locales: {}", e);
                std::process::exit(1);
            }
        }
    }


    let exe_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: Could not determine current executable path: {}", e);
            std::process::exit(1);
        }
    };

    let out_dir = workspace_root.join("generated_bindings");
    if let Err(e) = fs::create_dir_all(&out_dir) {
        eprintln!(
            "Error: Failed to create output directory '{:?}': {}",
            out_dir, e
        );
        std::process::exit(1);
    }

    let run_generation = |release: bool| -> Result<(), String> {
        compile_core(&workspace_root, release)?;
        check_wasm_compatibility(&workspace_root)?;
        let lib_path = find_library_path(&workspace_root, release)?;

        generate_bindings(&exe_path, &lib_path, "swift", &out_dir)?;
        generate_bindings(&exe_path, &lib_path, "kotlin", &out_dir)?;
        println!("Successfully generated all bindings!");
        Ok(())
    };

    if !is_watch {
        if let Err(e) = run_generation(is_release) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    } else {
        println!("Starting in watch mode on yntra-core/src/...");
        let core_src = workspace_root.join("yntra-core").join("src");
        if !core_src.exists() {
            eprintln!("Error: Core source directory '{:?}' not found.", core_src);
            std::process::exit(1);
        }

        // Initial run
        if let Err(e) = run_generation(is_release) {
            eprintln!("Initial generation failed: {}", e);
        }

        let mut last_mtime = get_max_mtime(&core_src);

        loop {
            thread::sleep(Duration::from_millis(1500));
            let current_mtime = get_max_mtime(&core_src);
            if current_mtime != last_mtime {
                println!("\nChanges detected in yntra-core/src/! Re-generating...");
                if let Err(e) = run_generation(is_release) {
                    eprintln!("Re-generation failed: {}", e);
                }
                last_mtime = current_mtime;
            }
        }
    }
}

fn find_workspace_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists()
            && let Ok(content) = fs::read_to_string(&cargo_toml)
            && content.contains("[workspace]")
        {
            return Some(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn compile_core(workspace_root: &Path, release: bool) -> Result<(), String> {
    println!("Compiling yntra-core...");
    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    cmd.arg("build");
    cmd.arg("-p");
    cmd.arg("yntra-core");
    if release {
        cmd.arg("--release");
    }

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to start cargo build: {}", e))?;
    if !status.success() {
        return Err("Cargo build of yntra-core failed".to_string());
    }
    Ok(())
}

fn check_wasm_compatibility(workspace_root: &Path) -> Result<(), String> {
    println!("Verifying target wasm32-unknown-unknown compatibility...");
    let mut cmd = Command::new("cargo");
    cmd.current_dir(workspace_root);
    cmd.arg("check");
    cmd.arg("--target");
    cmd.arg("wasm32-unknown-unknown");
    cmd.arg("-p");
    cmd.arg("yntra-core");

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to start cargo check for WASM: {}", e))?;
    if !status.success() {
        return Err("WASM target compatibility check failed. Ensure no native-only libraries or APIs are used in non-WASM configs.".to_string());
    }
    Ok(())
}

fn find_library_path(workspace_root: &Path, release: bool) -> Result<PathBuf, String> {
    let profile = if release { "release" } else { "debug" };
    let filename = match std::env::consts::OS {
        "windows" => "yntra_core.dll",
        "macos" => "libyntra_core.dylib",
        "linux" => "libyntra_core.so",
        other => {
            return Err(format!(
                "Unsupported OS for auto bindings generation: {}",
                other
            ));
        }
    };
    let path = workspace_root.join("target").join(profile).join(filename);
    if !path.exists() {
        return Err(format!("Compiled library not found at: {:?}", path));
    }
    Ok(path)
}

fn generate_bindings(
    exe_path: &Path,
    lib_path: &Path,
    language: &str,
    out_dir: &Path,
) -> Result<(), String> {
    println!("Generating bindings for {}...", language);
    let mut cmd = Command::new(exe_path);
    cmd.arg("generate");
    cmd.arg("--library");
    cmd.arg(lib_path);
    cmd.arg("--language");
    cmd.arg(language);
    cmd.arg("--out-dir");
    cmd.arg(out_dir);

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to run bindgen for {}: {}", language, e))?;
    if !status.success() {
        return Err(format!("Bindings generation failed for {}", language));
    }
    Ok(())
}

fn get_max_mtime(dir: &Path) -> Option<SystemTime> {
    let mut max_time = None;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(t) = get_max_mtime(&path) {
                    max_time = Some(max_time.map_or(t, |mt| std::cmp::max(mt, t)));
                }
            } else if path.is_file()
                && let Ok(metadata) = entry.metadata()
                && let Ok(modified) = metadata.modified()
            {
                max_time = Some(max_time.map_or(modified, |mt| std::cmp::max(mt, modified)));
            }
        }
    }
    max_time
}

fn install_hooks(workspace_root: &Path) -> Result<(), String> {
    let git_dir = workspace_root.join(".git");
    if !git_dir.exists() {
        return Err(
            "Not a git repository (could not find .git directory at workspace root)".to_string(),
        );
    }

    let hooks_dir = git_dir.join("hooks");
    fs::create_dir_all(&hooks_dir)
        .map_err(|e| format!("Failed to create git hooks directory: {}", e))?;

    let pre_commit_path = hooks_dir.join("pre-commit");

    let hook_content = r#"#!/bin/sh
# Automated WASM target compatibility check, localization validation, and UniFFI API parity pre-commit hook
echo "Checking yntra-core WASM target compatibility..."
cargo check --target wasm32-unknown-unknown -p yntra-core
if [ $? -ne 0 ]; then
    echo "Error: WASM target compilation check failed. Commit aborted."
    exit 1
fi

echo "Verifying translation catalogs..."
cargo run -p yntra-uniffi-bindgen -- check-locales
if [ $? -ne 0 ]; then
    echo "Error: Translation catalogs are inconsistent. Commit aborted."
    exit 1
fi

echo "Verifying UniFFI cross-platform API parity..."
cargo run -p yntra-uniffi-bindgen -- check-parity
if [ $? -ne 0 ]; then
    echo "Error: UniFFI cross-platform API parity check failed. Commit aborted."
    exit 1
fi
"#;

    fs::write(&pre_commit_path, hook_content)
        .map_err(|e| format!("Failed to write pre-commit hook file: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&pre_commit_path)
            .map_err(|e| format!("Failed to read metadata for pre-commit hook: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&pre_commit_path, perms).map_err(|e| {
            format!(
                "Failed to set executable permissions on pre-commit hook: {}",
                e
            )
        })?;
    }

    Ok(())
}

fn run_check_parity(workspace_root: &Path) -> Result<bool, String> {
    println!("Checking UniFFI exported API parity across client bindings...");
    let core_src = workspace_root.join("yntra-core").join("src");
    if !core_src.exists() {
        return Err(format!("Core source directory '{:?}' not found", core_src));
    }

    let mut exported_fns = Vec::new();
    collect_exported_functions(&core_src, &mut exported_fns)?;

    println!(
        "  -> Detected {} #[uniffi::export] symbols in yntra-core",
        exported_fns.len()
    );

    let bindings_dir = workspace_root.join("generated_bindings");
    if !bindings_dir.exists() {
        println!(
            "  -> Bindings directory '{:?}' does not exist yet. Run `cargo run -p yntra-uniffi-bindgen` to generate.",
            bindings_dir
        );
        return Ok(true);
    }

    let swift_file = bindings_dir.join("yntra_core.swift");
    let kt_file = bindings_dir
        .join("uniffi")
        .join("yntra_core")
        .join("yntra_core.kt");

    let mut missing_swift = Vec::new();
    let mut missing_kt = Vec::new();

    let swift_content = if swift_file.exists() {
        fs::read_to_string(&swift_file).unwrap_or_default()
    } else {
        String::new()
    };

    let kt_content = if kt_file.exists() {
        fs::read_to_string(&kt_file).unwrap_or_default()
    } else {
        String::new()
    };

    for fn_name in &exported_fns {
        let camel_name = to_camel_case(fn_name);
        if !swift_content.is_empty()
            && !swift_content.contains(&camel_name)
            && !swift_content.contains(fn_name)
        {
            missing_swift.push(fn_name.clone());
        }
        if !kt_content.is_empty()
            && !kt_content.contains(&camel_name)
            && !kt_content.contains(fn_name)
        {
            missing_kt.push(fn_name.clone());
        }
    }

    if missing_swift.is_empty() && missing_kt.is_empty() {
        println!(
            "  -> OK (All {} API symbols synchronized across Swift and Kotlin bindings)",
            exported_fns.len()
        );
        Ok(true)
    } else {
        if !missing_swift.is_empty() {
            println!(
                "  -> Warning: {} exported functions missing from Swift bindings: {:?}",
                missing_swift.len(),
                missing_swift
            );
        }
        if !missing_kt.is_empty() {
            println!(
                "  -> Warning: {} exported functions missing from Kotlin bindings: {:?}",
                missing_kt.len(),
                missing_kt
            );
        }
        println!("  -> Run `cargo run -p yntra-uniffi-bindgen` to regenerate bindings.");
        Ok(true)
    }
}

fn collect_exported_functions(dir: &Path, fns: &mut Vec<String>) -> Result<(), String> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_exported_functions(&path, fns)?;
            } else if path.extension().map_or(false, |ext| ext == "rs") {
                if let Ok(content) = fs::read_to_string(&path) {
                    let mut export_next = false;
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.contains("#[uniffi::export]") {
                            export_next = true;
                        } else if export_next
                            && (trimmed.starts_with("pub fn ") || trimmed.starts_with("pub async fn "))
                        {
                            let after_fn = if let Some(pos) = trimmed.find("fn ") {
                                &trimmed[pos + 3..]
                            } else {
                                ""
                            };
                            if let Some(paren_idx) = after_fn.find('(') {
                                let fn_name = after_fn[..paren_idx].trim().to_string();
                                if !fn_name.is_empty() && !fns.contains(&fn_name) {
                                    fns.push(fn_name);
                                }
                            }
                            export_next = false;
                        } else if export_next && !trimmed.starts_with('#') && !trimmed.is_empty() {
                            export_next = false;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for (_i, c) in s.chars().enumerate() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}


fn run_check_locales(workspace_root: &Path, fix: bool) -> Result<bool, String> {
    let locales_dir = workspace_root.join("yntra-ui").join("locales");
    if !locales_dir.exists() {
        return Err(format!("Locales directory not found at {:?}", locales_dir));
    }

    let en_path = locales_dir.join("en.ftl");
    if !en_path.exists() {
        return Err("English source translation file (en.ftl) is missing!".to_string());
    }

    let en_content =
        fs::read_to_string(&en_path).map_err(|e| format!("Failed to read en.ftl: {}", e))?;
    let en_map = parse_fluent_file(&en_content);

    let mut all_clean = true;

    let entries = fs::read_dir(&locales_dir)
        .map_err(|e| format!("Failed to read locales directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "ftl") {
            continue;
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name == "en.ftl" {
            continue;
        }

        println!("Checking translation catalog '{}'...", file_name);
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read file '{}': {}", file_name, e))?;
        let locale_map = parse_fluent_file(&content);

        let mut missing_keys = Vec::new();
        let mut extra_keys = Vec::new();
        let mut mismatching_vars = Vec::new();

        // 1. Check for missing and mismatching variables
        for (key, (en_val, en_vars)) in &en_map {
            match locale_map.get(key) {
                None => {
                    missing_keys.push((key.clone(), en_val.clone()));
                }
                Some((_loc_val, loc_vars)) => {
                    if en_vars != loc_vars {
                        mismatching_vars.push((key.clone(), en_vars.clone(), loc_vars.clone()));
                    }
                }
            }
        }

        // 2. Check for extra keys
        for key in locale_map.keys() {
            if !en_map.contains_key(key) {
                extra_keys.push(key.clone());
            }
        }

        // Reporting
        if missing_keys.is_empty() && extra_keys.is_empty() && mismatching_vars.is_empty() {
            println!("  -> OK ({} keys matched)", en_map.len());
            continue;
        }

        all_clean = false;
        println!("  -> Issues found in '{}':", file_name);

        if !missing_keys.is_empty() {
            println!("    * {} Missing Keys:", missing_keys.len());
            for (key, _) in &missing_keys {
                println!("      - {}", key);
            }
        }

        if !extra_keys.is_empty() {
            println!(
                "    * {} Extra Keys (not present in en.ftl):",
                extra_keys.len()
            );
            for key in &extra_keys {
                println!("      - {}", key);
            }
        }

        if !mismatching_vars.is_empty() {
            println!("    * {} Variable Mismatches:", mismatching_vars.len());
            for (key, en_vars, loc_vars) in &mismatching_vars {
                let en_vars_str = en_vars.iter().cloned().collect::<Vec<_>>().join(", ");
                let loc_vars_str = loc_vars.iter().cloned().collect::<Vec<_>>().join(", ");
                println!(
                    "      - {}: English variables = [{}], Localized variables = [{}]",
                    key, en_vars_str, loc_vars_str
                );
            }
        }

        // 3. Fix missing keys if `--fix` is passed
        if fix && !missing_keys.is_empty() {
            println!("    * Fixing missing keys in '{}'...", file_name);
            let mut append_content = String::new();
            append_content.push_str("\n\n# --- AUTO-ADDED MISSING TRANSLATIONS ---");
            for (key, en_val) in &missing_keys {
                append_content.push_str(&format!("\n{} = [TODO] {}", key, en_val));
            }

            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .map_err(|e| format!("Failed to open file '{}' for appending: {}", file_name, e))?;

            file.write_all(append_content.as_bytes()).map_err(|e| {
                format!(
                    "Failed to append missing translations to '{}': {}",
                    file_name, e
                )
            })?;

            println!(
                "      -> Successfully appended {} placeholders.",
                missing_keys.len()
            );
        }
    }

    Ok(all_clean || fix)
}

fn parse_fluent_file(content: &str) -> HashMap<String, (String, HashSet<String>)> {
    let mut map = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_value = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let first_char = line.chars().next().unwrap_or(' ');
        if first_char.is_ascii_alphabetic() {
            if let Some(ref key) = current_key {
                let vars = extract_variables(&current_value);
                map.insert(
                    key.clone(),
                    (current_value.clone(), vars.into_iter().collect()),
                );
                current_value.clear();
            }

            if let Some(eq_idx) = line.find('=') {
                let key = line[..eq_idx].trim().to_string();
                current_key = Some(key);
                current_value = line[eq_idx + 1..].trim().to_string();
            }
        } else if (line.starts_with(' ') || line.starts_with('\t')) && current_key.is_some() {
            if !current_value.is_empty() {
                current_value.push('\n');
            }
            current_value.push_str(line.trim());
        }
    }

    if let Some(ref key) = current_key {
        let vars = extract_variables(&current_value);
        map.insert(key.clone(), (current_value, vars.into_iter().collect()));
    }

    map
}

fn extract_variables(s: &str) -> Vec<String> {
    let mut vars = Vec::new();

    let mut rest = s;
    while let Some(start_idx) = rest.find("{{") {
        let after_braces = &rest[start_idx + 2..];
        if let Some(end_idx) = after_braces.find("}}") {
            let var_name = after_braces[..end_idx].trim().to_string();
            vars.push(var_name);
            rest = &after_braces[end_idx + 2..];
        } else {
            break;
        }
    }

    rest = s;
    while let Some(start_idx) = rest.find('{') {
        let after_brace = &rest[start_idx + 1..];
        if let Some(end_idx) = after_brace.find('}') {
            let inside = after_brace[..end_idx].trim();
            if let Some(clean) = inside.strip_prefix('$') {
                let var_name = clean.trim().to_string();
                vars.push(var_name);
            }
            rest = &after_brace[end_idx + 1..];
        } else {
            break;
        }
    }

    vars
}
