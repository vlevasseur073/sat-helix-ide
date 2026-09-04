use crate::config::{CommandConfig, TerminalConfig};
use std::path::Path;

pub fn session_layout(
    editor: &CommandConfig,
    editor_path: &Path,
    ai: Option<(&CommandConfig, &Path)>,
    terminal: Option<&TerminalConfig>,
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
    match terminal.filter(|terminal| terminal.enabled) {
        Some(terminal) => {
            layout.push_str(&format!(
                "        pane split_direction={direction:?} {{\n",
                direction = terminal.dock_position.zellij_split_direction()
            ));
            layout.push_str(&command_pane(
                "editor",
                editor_path,
                &editor.args,
                project_dir,
                3,
            ));
            layout.push_str(&format!(
                "            pane name=\"terminal\" size=\"{percent}%\" cwd={cwd:?}\n",
                percent = terminal.dock_percent,
                cwd = project_dir.display().to_string(),
            ));
            layout.push_str("        }\n");
        }
        None => layout.push_str(&command_pane(
            "editor",
            editor_path,
            &editor.args,
            project_dir,
            2,
        )),
    }
    layout.push_str("    }\n");

    if let Some((ai, ai_path)) = ai {
        layout.push_str("    tab name=\"ai\" {\n");
        layout.push_str(&command_pane("ai", ai_path, &ai.args, project_dir, 2));
        layout.push_str("    }\n");
    }
    layout.push_str("}\n");
    layout
}

fn command_pane(name: &str, command: &Path, args: &[String], cwd: &Path, indent: usize) -> String {
    let indentation = " ".repeat(indent * 4);
    let mut pane = format!(
        "{indentation}pane name={name:?} command={command:?} cwd={cwd:?}",
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
    use crate::config::TerminalDockPosition;

    fn terminal_config(percent: u8, position: TerminalDockPosition) -> TerminalConfig {
        TerminalConfig {
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
            Path::new("/work"),
            false,
        );

        assert!(layout.contains("tab name=\"code\" focus=true"));
        assert!(layout.contains("command=\"/usr/bin/hx\""));
        assert!(!layout.contains("tab name=\"ai\""));
        assert!(!layout.contains("name=\"terminal\""));
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
        let terminal = terminal_config(15, TerminalDockPosition::Down);
        let layout = session_layout(
            &editor,
            Path::new("/usr/bin/hx"),
            None,
            Some(&terminal),
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
        let terminal = terminal_config(28, TerminalDockPosition::Right);
        let layout = session_layout(
            &editor,
            Path::new("/usr/bin/hx"),
            None,
            Some(&terminal),
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
}
