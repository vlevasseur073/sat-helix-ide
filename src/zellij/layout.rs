use crate::config::{CommandConfig, MindMapConfig, TerminalConfig};
use std::path::Path;

/// Optional mind-map dock: config plus the resolved tool binary.
pub type MindMapLayout<'a> = (&'a MindMapConfig, &'a CommandConfig, &'a Path);

pub fn session_layout(
    editor: &CommandConfig,
    editor_path: &Path,
    ai: Option<(&CommandConfig, &Path)>,
    terminal: Option<&TerminalConfig>,
    mindmap: Option<MindMapLayout<'_>>,
    project_dir: &Path,
    status_bar: bool,
) -> String {
    let mut layout = String::from("layout {\n");
    if status_bar {
        layout.push_str("    tab_template name=\"status_tab\" {\n");
        layout.push_str("    children\n");
        layout.push_str("        pane size=2 borderless=true {\n");
        layout.push_str("            plugin location=\"zellij:status-bar\"\n");
        layout.push_str("        }\n");
        layout.push_str("    }\n");
    }
    let tab = if status_bar { "status_tab" } else { "tab" };
    layout.push_str(&format!("    {tab}"));
    layout.push_str(" name=\"code\" focus=true {\n");
    layout.push_str(&code_tab_panes(
        editor,
        editor_path,
        terminal.filter(|terminal| terminal.enabled),
        mindmap.filter(|(config, _, _)| config.enabled),
        project_dir,
    ));
    layout.push_str("    }\n");

    if let Some((ai, ai_path)) = ai {
        layout.push_str("    tab name=\"ai\" {\n");
        layout.push_str(&command_pane("ai", ai_path, &ai.args, project_dir, None, 2));
        layout.push_str("    }\n");
    }
    layout.push_str("}\n");
    layout
}

/// Nest the editor with optional mind-map and terminal docks.
///
/// When both docks are enabled, the mind-map wraps the editor first (so it
/// sits beside Helix), then the terminal wraps that pair.
fn code_tab_panes(
    editor: &CommandConfig,
    editor_path: &Path,
    terminal: Option<&TerminalConfig>,
    mindmap: Option<MindMapLayout<'_>>,
    project_dir: &Path,
) -> String {
    match (terminal, mindmap) {
        (None, None) => command_pane("editor", editor_path, &editor.args, project_dir, None, 2),
        (Some(terminal), None) => {
            let mut body = format!(
                "        pane split_direction={direction:?} {{\n",
                direction = terminal.dock_position.zellij_split_direction()
            );
            body.push_str(&command_pane(
                "editor",
                editor_path,
                &editor.args,
                project_dir,
                None,
                3,
            ));
            body.push_str(&shell_pane(
                "terminal",
                terminal.dock_percent,
                project_dir,
                3,
            ));
            body.push_str("        }\n");
            body
        }
        (None, Some((config, tool, path))) => {
            let mut body = format!(
                "        pane split_direction={direction:?} {{\n",
                direction = config.dock_position.zellij_split_direction()
            );
            body.push_str(&command_pane(
                "editor",
                editor_path,
                &editor.args,
                project_dir,
                None,
                3,
            ));
            body.push_str(&command_pane(
                "mindmapping",
                path,
                &tool.args,
                project_dir,
                Some(config.dock_percent),
                3,
            ));
            body.push_str("        }\n");
            body
        }
        (Some(terminal), Some((config, tool, path))) => {
            let mut body = format!(
                "        pane split_direction={direction:?} {{\n",
                direction = terminal.dock_position.zellij_split_direction()
            );
            body.push_str(&format!(
                "            pane split_direction={direction:?} {{\n",
                direction = config.dock_position.zellij_split_direction()
            ));
            body.push_str(&command_pane(
                "editor",
                editor_path,
                &editor.args,
                project_dir,
                None,
                4,
            ));
            body.push_str(&command_pane(
                "mindmapping",
                path,
                &tool.args,
                project_dir,
                Some(config.dock_percent),
                4,
            ));
            body.push_str("            }\n");
            body.push_str(&shell_pane(
                "terminal",
                terminal.dock_percent,
                project_dir,
                3,
            ));
            body.push_str("        }\n");
            body
        }
    }
}

fn shell_pane(name: &str, percent: u8, cwd: &Path, indent: usize) -> String {
    let indentation = " ".repeat(indent * 4);
    format!(
        "{indentation}pane name={name:?} size=\"{percent}%\" cwd={cwd:?}\n",
        cwd = cwd.display().to_string(),
    )
}

fn command_pane(
    name: &str,
    command: &Path,
    args: &[String],
    cwd: &Path,
    size_percent: Option<u8>,
    indent: usize,
) -> String {
    let indentation = " ".repeat(indent * 4);
    let size = size_percent
        .map(|percent| format!(" size=\"{percent}%\""))
        .unwrap_or_default();
    let mut pane = format!(
        "{indentation}pane name={name:?}{size} command={command:?} cwd={cwd:?}",
        command = command.display().to_string(),
        cwd = cwd.display().to_string(),
    );
    if args.is_empty() {
        pane.push('\n');
    } else {
        pane.push_str(" {\n");
        pane.push_str(&format!(
            "{indentation}    args {}\n",
            args.iter()
                .map(|arg| format!("{arg:?}"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
        pane.push_str(&format!("{indentation}}}\n"));
    }
    pane
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DockPosition;

    fn terminal_config(percent: u8, position: DockPosition) -> TerminalConfig {
        TerminalConfig {
            enabled: true,
            dock_percent: percent,
            dock_position: position,
        }
    }

    fn mindmap_config(percent: u8, position: DockPosition) -> MindMapConfig {
        MindMapConfig {
            enabled: true,
            dock_percent: percent,
            dock_position: position,
        }
    }

    #[test]
    fn builds_code_only_layout() {
        let editor = CommandConfig::new("hx");
        let layout = session_layout(
            &editor,
            Path::new("/usr/bin/hx"),
            None,
            None,
            None,
            Path::new("/work"),
            false,
        );

        assert!(layout.contains("tab name=\"code\" focus=true"));
        assert!(layout.contains("command=\"/usr/bin/hx\""));
        assert!(!layout.contains("tab name=\"ai\""));
        assert!(!layout.contains("name=\"terminal\""));
        assert!(!layout.contains("name=\"mindmapping\""));
    }

    #[test]
    fn adds_configured_ai_tab() {
        let editor = CommandConfig::new("hx");
        let ai = CommandConfig {
            command: "agent".to_string(),
            args: vec!["--resume".to_string()],
        };
        let layout = session_layout(
            &editor,
            Path::new("/usr/bin/hx"),
            Some((&ai, Path::new("/usr/bin/agent"))),
            None,
            None,
            Path::new("/work"),
            false,
        );

        assert!(layout.contains("tab name=\"ai\""));
        assert!(layout.contains("args \"--resume\""));
    }

    /// The terminal has no `command`, so Zellij starts its default shell.
    #[test]
    fn docks_the_terminal_under_the_editor() {
        let editor = CommandConfig::new("hx");
        let terminal = terminal_config(15, DockPosition::Down);
        let layout = session_layout(
            &editor,
            Path::new("/usr/bin/hx"),
            None,
            Some(&terminal),
            None,
            Path::new("/work"),
            false,
        );

        assert!(layout.contains("split_direction=\"horizontal\""));
        assert!(layout.contains("pane name=\"terminal\" size=\"15%\" cwd=\"/work\""));
        let editor_at = layout.find("name=\"editor\"").unwrap();
        let terminal_at = layout.find("name=\"terminal\"").unwrap();
        assert!(
            editor_at < terminal_at,
            "terminal must sit below the editor"
        );
    }

    #[test]
    fn docks_the_terminal_right_of_the_editor() {
        let editor = CommandConfig::new("hx");
        let terminal = terminal_config(28, DockPosition::Right);
        let layout = session_layout(
            &editor,
            Path::new("/usr/bin/hx"),
            None,
            Some(&terminal),
            None,
            Path::new("/work"),
            false,
        );

        assert!(layout.contains("split_direction=\"vertical\""));
        assert!(layout.contains("pane name=\"terminal\" size=\"28%\" cwd=\"/work\""));
        let editor_at = layout.find("name=\"editor\"").unwrap();
        let terminal_at = layout.find("name=\"terminal\"").unwrap();
        assert!(
            editor_at < terminal_at,
            "terminal must sit to the right of the editor"
        );
    }

    #[test]
    fn docks_the_mindmap_right_of_the_editor() {
        let editor = CommandConfig::new("hx");
        let mindmap = mindmap_config(40, DockPosition::Right);
        let tool = CommandConfig::new("shiki");
        let layout = session_layout(
            &editor,
            Path::new("/usr/bin/hx"),
            None,
            None,
            Some((&mindmap, &tool, Path::new("/usr/bin/shiki"))),
            Path::new("/work"),
            false,
        );

        assert!(layout.contains("split_direction=\"vertical\""));
        assert!(layout.contains(
            "pane name=\"mindmapping\" size=\"40%\" command=\"/usr/bin/shiki\" cwd=\"/work\""
        ));
        let editor_at = layout.find("name=\"editor\"").unwrap();
        let mindmap_at = layout.find("name=\"mindmapping\"").unwrap();
        assert!(
            editor_at < mindmap_at,
            "mind map must sit to the right of the editor"
        );
    }

    #[test]
    fn disabled_mindmap_is_omitted_from_layout() {
        let editor = CommandConfig::new("hx");
        let mut mindmap = mindmap_config(40, DockPosition::Right);
        mindmap.enabled = false;
        let tool = CommandConfig::new("shiki");
        let layout = session_layout(
            &editor,
            Path::new("/usr/bin/hx"),
            None,
            None,
            Some((&mindmap, &tool, Path::new("/usr/bin/shiki"))),
            Path::new("/work"),
            false,
        );

        assert!(!layout.contains("name=\"mindmapping\""));
    }

    #[test]
    fn nests_mindmap_inside_terminal_split() {
        let editor = CommandConfig::new("hx");
        let terminal = terminal_config(15, DockPosition::Down);
        let mindmap = mindmap_config(30, DockPosition::Right);
        let tool = CommandConfig::new("shiki");
        let layout = session_layout(
            &editor,
            Path::new("/usr/bin/hx"),
            None,
            Some(&terminal),
            Some((&mindmap, &tool, Path::new("/usr/bin/shiki"))),
            Path::new("/work"),
            false,
        );

        assert!(layout.contains("split_direction=\"horizontal\""));
        assert!(layout.contains("split_direction=\"vertical\""));
        assert!(layout.contains("name=\"mindmapping\""));
        assert!(layout.contains("name=\"terminal\""));
        let editor_at = layout.find("name=\"editor\"").unwrap();
        let mindmap_at = layout.find("name=\"mindmapping\"").unwrap();
        let terminal_at = layout.find("name=\"terminal\"").unwrap();
        assert!(editor_at < mindmap_at);
        assert!(mindmap_at < terminal_at);
    }
}
