use crate::config::KeybindingConfig;
use anyhow::{bail, Context, Result};
use kdl::{KdlDocument, KdlNode};
use std::fs;
use std::path::{Path, PathBuf};

pub struct RuntimeConfigInput<'a> {
    pub source: Option<&'a Path>,
    pub destination: &'a Path,
    pub session_name: &'a str,
    pub keys: &'a KeybindingConfig,
    pub executable: &'a Path,
    pub app_config: &'a Path,
    pub git_command: &'a Path,
    pub git_args: &'a [String],
    pub project_dir: &'a Path,
    pub float_width: &'a str,
    pub float_height: &'a str,
}

pub fn build_runtime_config(input: &RuntimeConfigInput<'_>) -> Result<PathBuf> {
    let source_text = match input.source {
        Some(path) => fs::read_to_string(path)
            .with_context(|| format!("Failed to read Zellij config {}", path.display()))?,
        None => String::new(),
    };
    let merged = merge_config(&source_text, input)?;

    if let Some(parent) = input.destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(input.destination, merged).with_context(|| {
        format!(
            "Failed to write private Zellij config {}",
            input.destination.display()
        )
    })?;
    Ok(input.destination.to_path_buf())
}

fn merge_config(source: &str, input: &RuntimeConfigInput<'_>) -> Result<String> {
    let mut document = if source.trim().is_empty() {
        KdlDocument::new()
    } else {
        source
            .parse::<KdlDocument>()
            .context("Failed to parse the existing Zellij config")?
    };

    let requested = [
        (&input.keys.file_manager, "file manager"),
        (&input.keys.file_manager_dock, "file-manager dock toggle"),
        (&input.keys.git, "Git client"),
        (&input.keys.review, "Review"),
        (&input.keys.workflow, "Workflow"),
        (&input.keys.terminal, "terminal toggle"),
        (&input.keys.terminal_zoom, "terminal zoom toggle"),
        (&input.keys.mindmap, "mind map toggle"),
        (&input.keys.mindmap_zoom, "mind map zoom toggle"),
    ];
    if let Some(keybinds) = document.get("keybinds") {
        for (key, action) in requested {
            if contains_binding(keybinds, key) {
                bail!(
                    "Zellij key '{key}' is already bound; choose another \
                     [keybindings] value for the sat-hx-ide {action} action"
                );
            }
        }
    }

    let bindings = [
        helper_binding(
            &input.keys.file_manager,
            input,
            &["__file-manager", "open"],
            "sat-file-manager-request",
            true,
        )?,
        helper_binding(
            &input.keys.file_manager_dock,
            input,
            &["__file-manager", "toggle-dock"],
            "sat-file-manager-toggle",
            false,
        )?,
        helper_binding(
            &input.keys.terminal,
            input,
            &["__terminal", "toggle"],
            "sat-terminal-toggle",
            false,
        )?,
        helper_binding(
            &input.keys.terminal_zoom,
            input,
            &["__terminal", "zoom"],
            "sat-terminal-zoom",
            false,
        )?,
        helper_binding(
            &input.keys.mindmap,
            input,
            &["__mindmap", "toggle"],
            "sat-mindmap-toggle",
            false,
        )?,
        helper_binding(
            &input.keys.mindmap_zoom,
            input,
            &["__mindmap", "zoom"],
            "sat-mindmap-zoom",
            false,
        )?,
        helper_binding(
            &input.keys.git,
            input,
            &["__git", "open"],
            "sat-git-open",
            false,
        )?,
        helper_binding(
            &input.keys.review,
            input,
            &["__review", "open"],
            "sat-review-open",
            false,
        )?,
        helper_binding(
            &input.keys.workflow,
            input,
            &["__workflow", "open"],
            "sat-workflow-open",
            false,
        )?,
    ];

    set_session_name(&mut document, input.session_name)?;
    set_session_serialization(&mut document, false)?;

    let keybinds = ensure_keybinds(&mut document);
    let shared = ensure_shared_except_locked(keybinds);
    let children = shared.ensure_children();
    children.nodes_mut().extend(bindings);

    Ok(document.to_string())
}

fn set_session_name(document: &mut KdlDocument, session_name: &str) -> Result<()> {
    document
        .nodes_mut()
        .retain(|node| node.name().value() != "session_name");
    document.nodes_mut().push(parse_single_node(&format!(
        "session_name {session_name:?}\n"
    ))?);
    Ok(())
}

fn set_session_serialization(document: &mut KdlDocument, enabled: bool) -> Result<()> {
    document
        .nodes_mut()
        .retain(|node| node.name().value() != "session_serialization");
    document.nodes_mut().push(parse_single_node(&format!(
        "session_serialization {}\n",
        enabled
    ))?);
    Ok(())
}

/// Runs sat-hx-ide itself in a floating pane. Helpers that only drive Zellij
/// actions and exit are given a 1x1 borderless pane so they stay invisible.
fn helper_binding(
    key: &str,
    input: &RuntimeConfigInput<'_>,
    args: &[&str],
    pane_name: &str,
    visible: bool,
) -> Result<KdlNode> {
    let (width, height, borderless) = if visible {
        (input.float_width, input.float_height, "")
    } else {
        ("1", "1", "\nborderless true")
    };
    let args = args
        .iter()
        .map(|arg| format!(" {arg:?}"))
        .collect::<String>();
    let snippet = format!(
        r#"bind {key:?} {{
    Run {exe:?} "--config" {config:?}{args} {{
        floating true
        close_on_exit true
        x "0%"
        y "0%"
        width {width:?}
        height {height:?}{borderless}
        name {pane_name:?}
        cwd {cwd:?}
    }}
}}"#,
        exe = input.executable.display().to_string(),
        config = input.app_config.display().to_string(),
        cwd = input.project_dir.display().to_string(),
    );
    parse_single_node(&snippet)
}

/// Nodes we insert carry no whitespace of their own so that the serializer
/// indents them for the depth they end up at, leaving the user's own
/// formatting untouched.
fn parse_single_node(source: &str) -> Result<KdlNode> {
    let mut node = source
        .parse::<KdlNode>()
        .context("Failed to build Zellij binding")?;
    node.clear_fmt_recursive();
    Ok(node)
}

fn ensure_keybinds(document: &mut KdlDocument) -> &mut KdlNode {
    let index = document
        .nodes()
        .iter()
        .position(|node| node.name().value() == "keybinds")
        .unwrap_or_else(|| {
            let node =
                parse_single_node("keybinds {\n}\n").expect("static keybinds KDL must be valid");
            document.nodes_mut().push(node);
            document.nodes().len() - 1
        });
    &mut document.nodes_mut()[index]
}

fn ensure_shared_except_locked(keybinds: &mut KdlNode) -> &mut KdlNode {
    let children = keybinds.ensure_children();
    let index = children
        .nodes()
        .iter()
        .position(|node| {
            node.name().value() == "shared_except"
                && node
                    .entries()
                    .iter()
                    .any(|entry| entry.value().as_string() == Some("locked"))
        })
        .unwrap_or_else(|| {
            let node = parse_single_node("shared_except \"locked\" {\n}\n")
                .expect("static shared keybind KDL must be valid");
            children.nodes_mut().push(node);
            children.nodes().len() - 1
        });
    &mut children.nodes_mut()[index]
}

fn contains_binding(node: &KdlNode, requested: &str) -> bool {
    let requested = normalize_key(requested);
    node.children().is_some_and(|children| {
        children.nodes().iter().any(|child| {
            (child.name().value() == "bind"
                && child.entries().iter().any(|entry| {
                    entry
                        .value()
                        .as_string()
                        .is_some_and(|key| normalize_key(key) == requested)
                }))
                || contains_binding(child, &requested)
        })
    })
}

fn normalize_key(key: &str) -> String {
    key.split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::KeybindingConfig;

    fn input<'a>(destination: &'a Path, keys: &'a KeybindingConfig) -> RuntimeConfigInput<'a> {
        RuntimeConfigInput {
            source: None,
            destination,
            session_name: "my-project",
            keys,
            executable: Path::new("/usr/bin/sat-hx-ide"),
            app_config: Path::new("/home/user/.config/sat-helix-ide/config.toml"),
            git_command: Path::new("/usr/bin/lazygit"),
            git_args: &[],
            project_dir: Path::new("/work/project"),
            float_width: "100%",
            float_height: "100%",
        }
    }

    #[test]
    fn preserves_source_file_and_adds_bindings_to_runtime_copy() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.kdl");
        let destination = temp.path().join("runtime/config.kdl");
        let original =
            "theme \"dracula\"\nkeybinds clear-defaults=true {\n    normal {\n    }\n}\n";
        fs::write(&source, original).unwrap();
        let keys = KeybindingConfig::default();
        let mut input = input(&destination, &keys);
        input.source = Some(&source);

        build_runtime_config(&input).unwrap();

        assert_eq!(fs::read_to_string(&source).unwrap(), original);
        let runtime = fs::read_to_string(destination).unwrap();
        assert!(runtime.contains("theme"));
        assert!(runtime.contains("dracula"));
        assert!(runtime.contains("bind \"Ctrl y\""));
        assert!(runtime.contains("bind \"Alt y\""));
        assert!(runtime.contains("bind \"Alt t\""));
        assert!(runtime.contains("bind \"Alt Shift t\""));
        assert!(runtime.contains("bind \"Alt m\""));
        assert!(runtime.contains("bind \"Alt Shift m\""));
        assert!(runtime.contains("bind \"Alt g\""));
        assert!(runtime.contains("bind \"Alt r\""));
        assert!(runtime.contains("bind \"Alt w\""));
        assert!(runtime.contains("__review"));
        assert!(runtime.contains("__mindmap"));
    }

    #[test]
    fn generated_bindings_reparse_inside_the_shared_section() {
        let keys = KeybindingConfig::default();
        let input = input(Path::new("/unused"), &keys);
        let source = "keybinds {\n    shared_except \"locked\" {\n        bind \"Ctrl p\" { Quit; }\n    }\n}\n";

        let merged = merge_config(source, &input).unwrap();

        let shared = merged
            .parse::<KdlDocument>()
            .expect("merged config must still be valid KDL")
            .get("keybinds")
            .and_then(|keybinds| keybinds.children())
            .and_then(|children| children.get("shared_except"))
            .and_then(|shared| shared.children())
            .expect("shared_except section must survive the merge")
            .nodes()
            .iter()
            .filter_map(|node| node.entries().first())
            .filter_map(|entry| entry.value().as_string())
            .map(str::to_owned)
            .collect::<Vec<_>>();

        assert_eq!(
            shared,
            [
                "Ctrl p",
                "Ctrl y",
                "Alt y",
                "Alt t",
                "Alt Shift t",
                "Alt m",
                "Alt Shift m",
                "Alt g",
                "Alt r",
                "Alt w"
            ]
        );
    }

    #[test]
    fn writes_project_session_name_into_runtime_config() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("runtime/config.kdl");
        let keys = KeybindingConfig::default();
        let input = input(&destination, &keys);

        build_runtime_config(&input).unwrap();

        let runtime = fs::read_to_string(destination).unwrap();
        assert!(runtime.contains("session_name \"my-project\""));
        assert!(runtime.contains("session_serialization false"));
    }

    #[test]
    fn rejects_an_existing_key() {
        let keys = KeybindingConfig::default();
        let input = input(Path::new("/unused"), &keys);
        let source = r#"
            keybinds {
                normal {
                    bind "Alt g" { Quit; }
                }
            }
        "#;

        let error = merge_config(source, &input).unwrap_err();

        assert!(error.to_string().contains("Alt g"));
    }
}
