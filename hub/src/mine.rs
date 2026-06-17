//! Mine catalog: scan the plugin's `mine/` kit (unregistered agents and skills),
//! compute install status against the project's `.claude/` dir, install items by
//! copying them in, and restore from a recorded `install.toml` manifest.
//!
//! All catalog logic lives here: PLUGIN_ROOT resolution, frontmatter parsing,
//! recursive copy, the TOML-backed install manifest, and the hub.toml proxy
//! config schema (load-only).

use crate::error::{HubError, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ---- data model -------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MineType {
    Skill,
    Agent,
}

impl MineType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MineType::Skill => "skill",
            MineType::Agent => "agent",
        }
    }

    pub fn parse(s: &str) -> Option<MineType> {
        match s {
            "skill" => Some(MineType::Skill),
            "agent" => Some(MineType::Agent),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MineItem {
    pub name: String,
    pub item_type: MineType,
    pub description: String,
    /// Absolute path of the item inside the mine. Internal only; never serialized.
    #[allow(dead_code)]
    pub source_path: PathBuf,
    pub installed: bool,
}

impl MineItem {
    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "type": self.item_type.as_str(),
            "description": self.description,
            "installed": self.installed,
        })
    }
}

// ---- PLUGIN_ROOT resolution -------------------------------------------------

/// Resolve the plugin root via a three-step cascade: `CLAUDE_PLUGIN_ROOT` env
/// var, then the CLI arg, then walk up from `current_exe()` (capped at 3
/// levels) looking for a dir containing `mine/`. Returns None if all fail.
pub fn resolve_plugin_root(cli_arg: Option<&Path>) -> Option<PathBuf> {
    if let Ok(env) = std::env::var("CLAUDE_PLUGIN_ROOT") {
        if !env.is_empty() {
            let p = PathBuf::from(env);
            if p.join("mine").is_dir() {
                return Some(p);
            }
        }
    }
    if let Some(arg) = cli_arg {
        if arg.join("mine").is_dir() {
            return Some(arg.to_path_buf());
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent();
        for _ in 0..3 {
            match cur {
                Some(dir) => {
                    if dir.join("mine").is_dir() {
                        return Some(dir.to_path_buf());
                    }
                    cur = dir.parent();
                }
                None => break,
            }
        }
    }
    None
}

// ---- frontmatter parsing ----------------------------------------------------

/// Extract `name` and `description` from a YAML frontmatter block. Handles the
/// `key: |` / `key: >` block-scalar form (indented continuation lines are
/// collapsed into the value). No YAML library required.
fn parse_frontmatter(content: &str) -> (Option<String>, Option<String>) {
    let mut lines = content.lines();
    // Frontmatter must open with `---`.
    match lines.next() {
        Some(first) if first.trim() == "---" => {}
        _ => return (None, None),
    }

    let mut name = None;
    let mut description = None;
    let mut block_key: Option<&str> = None;
    let mut block_buf: Vec<String> = Vec::new();

    let flush_block =
        |key: &str, buf: &[String], name: &mut Option<String>, desc: &mut Option<String>| {
            let joined = buf
                .iter()
                .map(|s| s.trim())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            match key {
                "name" => *name = Some(joined),
                "description" => *desc = Some(joined),
                _ => {}
            }
        };

    for line in lines {
        if line.trim() == "---" {
            if let Some(key) = block_key.take() {
                flush_block(key, &block_buf, &mut name, &mut description);
                block_buf.clear();
            }
            break;
        }

        // Inside a block scalar: any indented (or blank) line continues it.
        if let Some(key) = block_key {
            let is_continuation = line.is_empty() || line.starts_with(char::is_whitespace);
            if is_continuation {
                block_buf.push(line.to_string());
                continue;
            }
            // Dedent back to a top-level key: close the block first.
            flush_block(key, &block_buf, &mut name, &mut description);
            block_buf.clear();
            block_key = None;
        }

        // Only top-level keys (no leading whitespace) matter here.
        if line.starts_with(char::is_whitespace) {
            continue;
        }
        let Some((raw_key, raw_val)) = line.split_once(':') else {
            continue;
        };
        let key = raw_key.trim();
        if key != "name" && key != "description" {
            continue;
        }
        let val = raw_val.trim();
        if val == "|" || val == ">" || val == "|-" || val == ">-" || val == "|+" || val == ">+" {
            block_key = match key {
                "name" => Some("name"),
                "description" => Some("description"),
                _ => None,
            };
            continue;
        }
        let cleaned = strip_quotes(val);
        match key {
            "name" => name = Some(cleaned),
            "description" => description = Some(cleaned),
            _ => {}
        }
    }

    (name, description)
}

fn strip_quotes(s: &str) -> String {
    let t = s.trim();
    if (t.starts_with('"') && t.ends_with('"') && t.len() >= 2)
        || (t.starts_with('\'') && t.ends_with('\'') && t.len() >= 2)
    {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

// ---- catalog scan -----------------------------------------------------------

/// Scan `<plugin_root>/mine/agents/*.md` and `<plugin_root>/mine/skills/*/SKILL.md`
/// into a catalog. Install status is computed fresh against `project_cwd`.
/// Malformed or unreadable entries are skipped with a stderr warning.
pub fn scan_catalog(plugin_root: &Path, project_cwd: &Path) -> Vec<MineItem> {
    let mut items = Vec::new();

    let agents_dir = plugin_root.join("mine").join("agents");
    if let Ok(entries) = fs::read_dir(&agents_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") || !path.is_file() {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let name = stem.to_string();
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("hub mine: skip agent {}: {e}", path.display());
                    continue;
                }
            };
            let (_pname, pdesc) = parse_frontmatter(&content);
            let installed = project_cwd
                .join(".claude")
                .join("agents")
                .join(format!("{name}.md"))
                .exists();
            items.push(MineItem {
                name,
                item_type: MineType::Agent,
                description: pdesc.unwrap_or_default(),
                source_path: path,
                installed,
            });
        }
    }

    let skills_dir = plugin_root.join("mine").join("skills");
    if let Ok(entries) = fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let skill_md = path.join("SKILL.md");
            if !skill_md.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let name = name.to_string();
            let content = match fs::read_to_string(&skill_md) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("hub mine: skip skill {}: {e}", skill_md.display());
                    continue;
                }
            };
            let (_pname, pdesc) = parse_frontmatter(&content);
            let installed = project_cwd
                .join(".claude")
                .join("skills")
                .join(&name)
                .is_dir();
            items.push(MineItem {
                name,
                item_type: MineType::Skill,
                description: pdesc.unwrap_or_default(),
                source_path: path,
                installed,
            });
        }
    }

    items
}

/// Catalog as a sorted JSON `{ "items": [...] }`: agents before skills, each
/// group lexicographic by name.
pub fn list_json(plugin_root: &Path, project_cwd: &Path) -> Value {
    let mut items = scan_catalog(plugin_root, project_cwd);
    items.sort_by(|a, b| {
        let rank = |t: MineType| match t {
            MineType::Agent => 0,
            MineType::Skill => 1,
        };
        rank(a.item_type)
            .cmp(&rank(b.item_type))
            .then_with(|| a.name.cmp(&b.name))
    });
    json!({ "items": items.iter().map(|i| i.to_json()).collect::<Vec<_>>() })
}

// ---- recursive copy ---------------------------------------------------------

/// Recursively copy a directory tree using only the standard library.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)
        .map_err(|e| HubError::new(format!("create dir {}: {e}", dst.display())))?;
    for entry in
        fs::read_dir(src).map_err(|e| HubError::new(format!("read dir {}: {e}", src.display())))?
    {
        let entry = entry.map_err(|e| HubError::new(format!("read entry: {e}")))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| {
                HubError::new(format!("copy {} -> {}: {e}", from.display(), to.display()))
            })?;
        }
    }
    Ok(())
}

// ---- install manifest -------------------------------------------------------

#[derive(Debug, Default, PartialEq, Eq)]
pub struct InstallManifest {
    pub skills_installed: Vec<String>,
    pub agents_installed: Vec<String>,
}

impl InstallManifest {
    fn path(project_cwd: &Path) -> PathBuf {
        project_cwd.join(".machine").join("install.toml")
    }

    /// Load the manifest. Absent file yields an empty manifest; malformed TOML
    /// is an error surfaced with the path.
    pub fn load(project_cwd: &Path) -> Result<InstallManifest> {
        let path = Self::path(project_cwd);
        if !path.exists() {
            return Ok(InstallManifest::default());
        }
        let text = fs::read_to_string(&path)
            .map_err(|e| HubError::new(format!("read {}: {e}", path.display())))?;
        let value: toml::Value = toml::from_str(&text)
            .map_err(|e| HubError::new(format!("parse {}: {e}", path.display())))?;

        let read_list = |table: &str| -> Vec<String> {
            value
                .get(table)
                .and_then(|t| t.get("installed"))
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        };

        Ok(InstallManifest {
            skills_installed: read_list("skills"),
            agents_installed: read_list("agents"),
        })
    }

    /// Append `name` under `item_type` if not already present, then persist
    /// atomically (temp file + rename). Creates `.machine/` if absent.
    pub fn append(project_cwd: &Path, item_type: MineType, name: &str) -> Result<()> {
        let mut manifest = Self::load(project_cwd)?;
        let list = match item_type {
            MineType::Skill => &mut manifest.skills_installed,
            MineType::Agent => &mut manifest.agents_installed,
        };
        if list.iter().any(|n| n == name) {
            return Ok(());
        }
        list.push(name.to_string());
        manifest.save(project_cwd)
    }

    fn save(&self, project_cwd: &Path) -> Result<()> {
        let dir = project_cwd.join(".machine");
        fs::create_dir_all(&dir)
            .map_err(|e| HubError::new(format!("create dir {}: {e}", dir.display())))?;

        let mut root = toml::value::Table::new();
        let mut skills = toml::value::Table::new();
        skills.insert(
            "installed".into(),
            toml::Value::Array(
                self.skills_installed
                    .iter()
                    .map(|s| toml::Value::String(s.clone()))
                    .collect(),
            ),
        );
        let mut agents = toml::value::Table::new();
        agents.insert(
            "installed".into(),
            toml::Value::Array(
                self.agents_installed
                    .iter()
                    .map(|s| toml::Value::String(s.clone()))
                    .collect(),
            ),
        );
        root.insert("skills".into(), toml::Value::Table(skills));
        root.insert("agents".into(), toml::Value::Table(agents));

        let text = toml::to_string_pretty(&toml::Value::Table(root))
            .map_err(|e| HubError::new(format!("serialize install.toml: {e}")))?;

        let final_path = Self::path(project_cwd);
        let tmp_path = dir.join("install.toml.tmp");
        fs::write(&tmp_path, text.as_bytes())
            .map_err(|e| HubError::new(format!("write {}: {e}", tmp_path.display())))?;
        fs::rename(&tmp_path, &final_path)
            .map_err(|e| HubError::new(format!("rename install.toml: {e}")))?;
        Ok(())
    }
}

// ---- install ----------------------------------------------------------------

/// Install a catalog item by copying it into the project's `.claude/`, then
/// recording it in `install.toml`. Idempotent: an already-installed item
/// returns `{ "status": "already_installed", ... }` without copying.
pub fn install_item(
    plugin_root: &Path,
    project_cwd: &Path,
    item_type: MineType,
    name: &str,
) -> Result<Value> {
    let catalog = scan_catalog(plugin_root, project_cwd);
    let item = catalog
        .iter()
        .find(|i| i.item_type == item_type && i.name == name)
        .ok_or_else(|| {
            HubError::new(format!(
                "mine item not found: {} {name}",
                item_type.as_str()
            ))
        })?;

    if item.installed {
        return Ok(
            json!({ "status": "already_installed", "name": name, "type": item_type.as_str() }),
        );
    }

    match item_type {
        MineType::Agent => {
            let src = plugin_root
                .join("mine")
                .join("agents")
                .join(format!("{name}.md"));
            let dst_dir = project_cwd.join(".claude").join("agents");
            fs::create_dir_all(&dst_dir)
                .map_err(|e| HubError::new(format!("create dir {}: {e}", dst_dir.display())))?;
            let dst = dst_dir.join(format!("{name}.md"));
            fs::copy(&src, &dst).map_err(|e| {
                HubError::new(format!("copy {} -> {}: {e}", src.display(), dst.display()))
            })?;
        }
        MineType::Skill => {
            let src = plugin_root.join("mine").join("skills").join(name);
            let dst = project_cwd.join(".claude").join("skills").join(name);
            copy_dir_recursive(&src, &dst)?;
        }
    }

    // Files are the source of truth: a manifest append failure logs but does
    // not reverse the copy. Registry auto-registration is a future extension.
    if let Err(e) = InstallManifest::append(project_cwd, item_type, name) {
        eprintln!("hub mine: install.toml append failed for {name}: {e}");
    }

    Ok(json!({ "status": "installed", "name": name, "type": item_type.as_str() }))
}

// ---- restore ----------------------------------------------------------------

/// Idempotently restore every manifest-listed item that is missing from the
/// project's `.claude/`. Items already present are skipped (never overwritten);
/// items missing from the mine are skipped with a warning. Returns counts.
pub fn mine_restore(plugin_root: &Path, project_cwd: &Path) -> Result<Value> {
    let manifest = InstallManifest::load(project_cwd)?;
    let mut restored = 0u64;
    let mut skipped = 0u64;

    for name in &manifest.agents_installed {
        let dst = project_cwd
            .join(".claude")
            .join("agents")
            .join(format!("{name}.md"));
        if dst.exists() {
            skipped += 1;
            continue;
        }
        let src = plugin_root
            .join("mine")
            .join("agents")
            .join(format!("{name}.md"));
        if !src.is_file() {
            eprintln!("hub mine: restore skip agent {name} — source missing in mine");
            skipped += 1;
            continue;
        }
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| HubError::new(format!("create dir {}: {e}", parent.display())))?;
        }
        match fs::copy(&src, &dst) {
            Ok(_) => restored += 1,
            Err(e) => {
                eprintln!("hub mine: restore agent {name} failed: {e}");
                skipped += 1;
            }
        }
    }

    for name in &manifest.skills_installed {
        let dst = project_cwd.join(".claude").join("skills").join(name);
        if dst.exists() {
            skipped += 1;
            continue;
        }
        let src = plugin_root.join("mine").join("skills").join(name);
        if !src.is_dir() {
            eprintln!("hub mine: restore skip skill {name} — source missing in mine");
            skipped += 1;
            continue;
        }
        match copy_dir_recursive(&src, &dst) {
            Ok(_) => restored += 1,
            Err(e) => {
                eprintln!("hub mine: restore skill {name} failed: {e}");
                skipped += 1;
            }
        }
    }

    Ok(json!({ "restored": restored, "skipped": skipped }))
}

// ---- hub.toml proxy config (load-only) --------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct McpEntry {
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env: HashMap<String, String>,
}

#[derive(Clone, Debug, Default)]
pub struct HubTomlConfig {
    pub mcps: HashMap<String, McpEntry>,
}

/// Load the hub.toml proxy config. Resolution order: `<plugin_root>/hub.toml`,
/// then `<project_cwd>/hub.toml`, then `<project_cwd>/.machine/hub.toml`; first
/// found wins. No file found or a parse error yields an empty config (the error
/// is logged to stderr). Load-only — no proxy behavior is implemented here.
pub fn load_hub_toml(plugin_root: &Path, project_cwd: &Path) -> HubTomlConfig {
    let candidates = [
        plugin_root.join("hub.toml"),
        project_cwd.join("hub.toml"),
        project_cwd.join(".machine").join("hub.toml"),
    ];
    let path = candidates.into_iter().find(|p| p.is_file());
    let Some(path) = path else {
        return HubTomlConfig::default();
    };

    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("hub.toml: read {} failed: {e}", path.display());
            return HubTomlConfig::default();
        }
    };
    let value: toml::Value = match toml::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("hub.toml: parse {} failed: {e}", path.display());
            return HubTomlConfig::default();
        }
    };

    let mut config = HubTomlConfig::default();
    let Some(mcps) = value.get("mcps").and_then(|m| m.as_table()) else {
        return config;
    };

    for (name, raw) in mcps {
        let Some(table) = raw.as_table() else {
            eprintln!("hub.toml: mcp '{name}' is not a table — skipped");
            continue;
        };
        let command = table
            .get("command")
            .and_then(|v| v.as_str())
            .map(String::from);
        let url = table.get("url").and_then(|v| v.as_str()).map(String::from);
        if command.is_some() && url.is_some() {
            eprintln!("hub.toml: mcp '{name}' sets both command and url — skipped");
            continue;
        }
        let args = table
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let env = table
            .get("env")
            .and_then(|v| v.as_table())
            .map(|t| {
                t.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        config.mcps.insert(
            name.clone(),
            McpEntry {
                command,
                args,
                url,
                env,
            },
        );
    }

    config
}

// ---- tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    /// Build a synthetic mine: two agents, two skills.
    fn make_mine(root: &Path) {
        write(
            &root.join("mine/agents/alpha.md"),
            "---\nname: alpha\ndescription: First agent.\n---\nbody",
        );
        write(
            &root.join("mine/agents/beta.md"),
            "---\nname: beta\ndescription: |\n  Second agent.\n  Multi line.\n---\nbody",
        );
        write(
            &root.join("mine/skills/one/SKILL.md"),
            "---\nname: one\ndescription: Skill one.\n---\nbody",
        );
        write(
            &root.join("mine/skills/two/SKILL.md"),
            "---\nname: two\ndescription: Skill two.\n---\nbody",
        );
        // Nested file in a skill to exercise recursive copy.
        write(&root.join("mine/skills/two/lib/helper.txt"), "deep");
    }

    #[test]
    fn scan_returns_four_items_with_metadata() {
        let plugin = tempdir().unwrap();
        let cwd = tempdir().unwrap();
        make_mine(plugin.path());

        let items = scan_catalog(plugin.path(), cwd.path());
        assert_eq!(items.len(), 4);

        let alpha = items.iter().find(|i| i.name == "alpha").unwrap();
        assert_eq!(alpha.item_type, MineType::Agent);
        assert_eq!(alpha.description, "First agent.");
        assert!(!alpha.installed);

        let beta = items.iter().find(|i| i.name == "beta").unwrap();
        assert_eq!(beta.description, "Second agent. Multi line.");

        let one = items.iter().find(|i| i.name == "one").unwrap();
        assert_eq!(one.item_type, MineType::Skill);
        assert_eq!(one.description, "Skill one.");
    }

    #[test]
    fn scan_marks_installed_when_claude_path_exists() {
        let plugin = tempdir().unwrap();
        let cwd = tempdir().unwrap();
        make_mine(plugin.path());
        write(&cwd.path().join(".claude/agents/alpha.md"), "installed");
        fs::create_dir_all(cwd.path().join(".claude/skills/one")).unwrap();

        let items = scan_catalog(plugin.path(), cwd.path());
        assert!(items.iter().find(|i| i.name == "alpha").unwrap().installed);
        assert!(items.iter().find(|i| i.name == "one").unwrap().installed);
        assert!(!items.iter().find(|i| i.name == "beta").unwrap().installed);
    }

    #[test]
    fn scan_skips_malformed_without_panic() {
        let plugin = tempdir().unwrap();
        let cwd = tempdir().unwrap();
        // Agent with no frontmatter — still listed, empty description.
        write(
            &plugin.path().join("mine/agents/raw.md"),
            "no frontmatter here",
        );
        // Skill dir with no SKILL.md — skipped entirely.
        fs::create_dir_all(plugin.path().join("mine/skills/empty")).unwrap();

        let items = scan_catalog(plugin.path(), cwd.path());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "raw");
        assert_eq!(items[0].description, "");
    }

    #[test]
    fn list_json_sorts_agents_before_skills() {
        let plugin = tempdir().unwrap();
        let cwd = tempdir().unwrap();
        make_mine(plugin.path());

        let v = list_json(plugin.path(), cwd.path());
        let arr = v["items"].as_array().unwrap();
        let names: Vec<&str> = arr.iter().map(|i| i["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["alpha", "beta", "one", "two"]);
        assert!(arr[0].get("type").is_some());
        // source_path never leaks into JSON.
        assert!(arr[0].get("source_path").is_none());
    }

    #[test]
    fn manifest_round_trip_and_idempotent_append() {
        let cwd = tempdir().unwrap();
        InstallManifest::append(cwd.path(), MineType::Agent, "alpha").unwrap();
        InstallManifest::append(cwd.path(), MineType::Agent, "alpha").unwrap();
        InstallManifest::append(cwd.path(), MineType::Skill, "one").unwrap();

        let m = InstallManifest::load(cwd.path()).unwrap();
        assert_eq!(m.agents_installed, vec!["alpha"]);
        assert_eq!(m.skills_installed, vec!["one"]);
    }

    #[test]
    fn manifest_missing_file_is_empty() {
        let cwd = tempdir().unwrap();
        let m = InstallManifest::load(cwd.path()).unwrap();
        assert_eq!(m, InstallManifest::default());
    }

    #[test]
    fn manifest_corrupt_toml_errors() {
        let cwd = tempdir().unwrap();
        write(
            &cwd.path().join(".machine/install.toml"),
            "this is = = not toml",
        );
        assert!(InstallManifest::load(cwd.path()).is_err());
    }

    #[test]
    fn install_copies_agent_and_skill_recursively() {
        let plugin = tempdir().unwrap();
        let cwd = tempdir().unwrap();
        make_mine(plugin.path());

        let r = install_item(plugin.path(), cwd.path(), MineType::Agent, "alpha").unwrap();
        assert_eq!(r["status"], "installed");
        assert!(cwd.path().join(".claude/agents/alpha.md").is_file());

        let r2 = install_item(plugin.path(), cwd.path(), MineType::Skill, "two").unwrap();
        assert_eq!(r2["status"], "installed");
        assert!(cwd.path().join(".claude/skills/two/SKILL.md").is_file());
        assert!(cwd
            .path()
            .join(".claude/skills/two/lib/helper.txt")
            .is_file());

        // Second install is a no-op.
        let again = install_item(plugin.path(), cwd.path(), MineType::Agent, "alpha").unwrap();
        assert_eq!(again["status"], "already_installed");

        let m = InstallManifest::load(cwd.path()).unwrap();
        assert_eq!(m.agents_installed, vec!["alpha"]);
        assert_eq!(m.skills_installed, vec!["two"]);
    }

    #[test]
    fn install_unknown_item_errors() {
        let plugin = tempdir().unwrap();
        let cwd = tempdir().unwrap();
        make_mine(plugin.path());
        assert!(install_item(plugin.path(), cwd.path(), MineType::Agent, "ghost").is_err());
    }

    #[test]
    fn restore_copies_missing_and_skips_present() {
        let plugin = tempdir().unwrap();
        let cwd = tempdir().unwrap();
        make_mine(plugin.path());

        // Manifest lists one agent + one skill; the agent already exists.
        write(
            &cwd.path().join(".machine/install.toml"),
            "[skills]\ninstalled = [\"one\"]\n[agents]\ninstalled = [\"alpha\"]\n",
        );
        write(&cwd.path().join(".claude/agents/alpha.md"), "present");

        let r = mine_restore(plugin.path(), cwd.path()).unwrap();
        assert_eq!(r["restored"], 1); // skill "one" copied
        assert_eq!(r["skipped"], 1); // agent "alpha" already present
        assert!(cwd.path().join(".claude/skills/one/SKILL.md").is_file());

        // Idempotent second pass: nothing left to restore.
        let r2 = mine_restore(plugin.path(), cwd.path()).unwrap();
        assert_eq!(r2["restored"], 0);
        assert_eq!(r2["skipped"], 2);
    }

    #[test]
    fn restore_skips_source_missing_from_mine() {
        let plugin = tempdir().unwrap();
        let cwd = tempdir().unwrap();
        make_mine(plugin.path());
        write(
            &cwd.path().join(".machine/install.toml"),
            "[skills]\ninstalled = []\n[agents]\ninstalled = [\"ghost\"]\n",
        );
        let r = mine_restore(plugin.path(), cwd.path()).unwrap();
        assert_eq!(r["restored"], 0);
        assert_eq!(r["skipped"], 1);
    }

    #[test]
    fn hub_toml_parses_stdio_and_http_entries() {
        let plugin = tempdir().unwrap();
        let cwd = tempdir().unwrap();
        write(
            &plugin.path().join("hub.toml"),
            "[mcps.local]\ncommand = \"node\"\nargs = [\"server.js\"]\n\n[mcps.local.env]\nKEY = \"val\"\n\n[mcps.remote]\nurl = \"https://example.com/mcp\"\n",
        );
        let cfg = load_hub_toml(plugin.path(), cwd.path());
        assert_eq!(cfg.mcps.len(), 2);
        let local = &cfg.mcps["local"];
        assert_eq!(local.command.as_deref(), Some("node"));
        assert_eq!(local.args, vec!["server.js"]);
        assert_eq!(local.env.get("KEY").map(String::as_str), Some("val"));
        let remote = &cfg.mcps["remote"];
        assert_eq!(remote.url.as_deref(), Some("https://example.com/mcp"));
    }

    #[test]
    fn hub_toml_rejects_mixed_command_and_url() {
        let plugin = tempdir().unwrap();
        let cwd = tempdir().unwrap();
        write(
            &plugin.path().join("hub.toml"),
            "[mcps.bad]\ncommand = \"node\"\nurl = \"https://x\"\n",
        );
        let cfg = load_hub_toml(plugin.path(), cwd.path());
        assert_eq!(cfg.mcps.len(), 0);
    }

    #[test]
    fn hub_toml_missing_is_empty() {
        let plugin = tempdir().unwrap();
        let cwd = tempdir().unwrap();
        let cfg = load_hub_toml(plugin.path(), cwd.path());
        assert_eq!(cfg.mcps.len(), 0);
    }
}
