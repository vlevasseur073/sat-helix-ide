use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Layout configuration for Zellij
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Layout name
    pub name: String,

    /// Layout description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Layout direction (vertical or horizontal)
    #[serde(default = "default_layout_direction")]
    pub direction: String,

    /// Layout parts
    #[serde(default)]
    pub parts: Vec<LayoutPart>,

    /// Default pane sizes
    #[serde(default)]
    pub sizes: Option<Vec<LayoutSize>>,

    /// Theme for this layout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,

    /// Keybindings for this layout
    #[serde(default)]
    pub keybinds: HashMap<String, String>,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            description: None,
            direction: default_layout_direction(),
            parts: vec![],
            sizes: None,
            theme: None,
            keybinds: HashMap::new(),
        }
    }
}

fn default_layout_direction() -> String {
    "vertical".to_string()
}

/// Layout part definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPart {
    /// Part direction
    #[serde(default = "default_part_direction")]
    pub direction: String,

    /// Whether this part is borderless.
    ///
    /// Zellij rejects `borderless` on a pane that has nested panes, so this is
    /// applied to the leaf panes contained in this part.
    #[serde(default)]
    pub borderless: bool,

    /// Panes in this part
    #[serde(default)]
    pub panes: Vec<LayoutPane>,

    /// Parts within this part
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<LayoutPart>,

    /// Size of this part
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<LayoutSize>,
}

impl Default for LayoutPart {
    fn default() -> Self {
        Self {
            direction: default_part_direction(),
            borderless: false,
            panes: vec![],
            parts: vec![],
            size: None,
        }
    }
}

fn default_part_direction() -> String {
    "horizontal".to_string()
}

/// Layout pane definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPane {
    /// Pane ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Command to run in this pane
    #[serde(default)]
    pub command: String,

    /// Arguments passed to the pane command
    #[serde(default)]
    pub args: Vec<String>,

    /// Whether to run the command
    #[serde(default = "default_pane_run")]
    pub run: bool,

    /// Pane-specific configuration
    #[serde(default)]
    pub config: HashMap<String, String>,
}

impl Default for LayoutPane {
    fn default() -> Self {
        Self {
            id: None,
            command: String::new(),
            args: vec![],
            run: default_pane_run(),
            config: HashMap::new(),
        }
    }
}

fn default_pane_run() -> bool {
    true
}

/// Layout size definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSize {
    /// Size type (Percentage or Fixed)
    #[serde(default = "default_size_type")]
    pub r#type: String,

    /// Size value
    pub value: f32,
}

impl Default for LayoutSize {
    fn default() -> Self {
        Self {
            r#type: default_size_type(),
            value: 50.0,
        }
    }
}

fn default_size_type() -> String {
    "Percentage".to_string()
}

impl LayoutConfig {
    /// Create a new layout with default settings
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Add a pane to the layout
    pub fn add_pane(mut self, command: &str) -> Self {
        if self.parts.is_empty() {
            self.parts.push(LayoutPart::default());
        }
        self.parts[0].panes.push(LayoutPane {
            command: command.to_string(),
            ..Default::default()
        });
        self
    }

    /// Generate Zellij KDL layout
    pub fn to_kdl(&self) -> String {
        let mut kdl = String::new();
        kdl.push_str(&format!("// {} layout\n", self.name));
        if let Some(desc) = &self.description {
            kdl.push_str(&format!("// {}\n", desc));
        }
        kdl.push_str("layout {\n");
        if self.parts.len() == 1 {
            kdl.push_str(&Self::generate_part_kdl(&self.parts[0], 1, false));
        } else if !self.parts.is_empty() {
            kdl.push_str(&format!(
                "    pane split_direction=\"{}\" {{\n",
                Self::escape_kdl(&self.direction)
            ));
            for part in &self.parts {
                kdl.push_str(&Self::generate_part_kdl(part, 2, false));
            }
            kdl.push_str("    }\n");
        } else {
            kdl.push_str("    pane\n");
        }
        kdl.push_str("}\n");
        kdl
    }

    fn generate_part_kdl(part: &LayoutPart, indent: usize, inherited_borderless: bool) -> String {
        let mut kdl = String::new();
        let indent_str = " ".repeat(indent * 4);
        let borderless = part.borderless || inherited_borderless;

        kdl.push_str(&format!(
            "{}pane split_direction=\"{}\"",
            indent_str,
            Self::escape_kdl(&part.direction)
        ));
        if let Some(size) = &part.size {
            match size.r#type.to_ascii_lowercase().as_str() {
                "percentage" => kdl.push_str(&format!(" size=\"{}%\"", size.value)),
                "fixed" => kdl.push_str(&format!(" size={}", size.value)),
                _ => {}
            }
        }
        kdl.push_str(" {\n");

        for pane in &part.panes {
            kdl.push_str(&Self::generate_pane_kdl(pane, indent + 1, borderless));
        }
        for nested in &part.parts {
            kdl.push_str(&Self::generate_part_kdl(nested, indent + 1, borderless));
        }
        kdl.push_str(&format!("{}}}\n", indent_str));

        kdl
    }

    fn generate_pane_kdl(pane: &LayoutPane, indent: usize, borderless: bool) -> String {
        let indent_str = " ".repeat(indent * 4);
        let mut attributes = String::new();

        if let Some(name) = &pane.id {
            attributes.push_str(&format!(" name=\"{}\"", Self::escape_kdl(name)));
        }
        if borderless {
            attributes.push_str(" borderless=true");
        }

        let mut command_parts = pane.command.split_whitespace();
        let command = command_parts.next();
        if pane.run {
            if let Some(command) = command {
                attributes.push_str(&format!(" command=\"{}\"", Self::escape_kdl(command)));
                let mut args: Vec<_> = command_parts.map(str::to_owned).collect();
                args.extend(pane.args.iter().cloned());
                if !args.is_empty() {
                    let args = args
                        .iter()
                        .map(|arg| format!("\"{}\"", Self::escape_kdl(arg)))
                        .collect::<Vec<_>>()
                        .join(" ");
                    return format!(
                        "{}pane{} {{\n{}    args {}\n{}}}\n",
                        indent_str, attributes, indent_str, args, indent_str
                    );
                }
            }
        }

        format!("{}pane{}\n", indent_str, attributes)
    }

    fn escape_kdl(value: &str) -> String {
        value.replace('\\', "\\\\").replace('"', "\\\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_current_zellij_layout_syntax() {
        let layout = LayoutConfig {
            name: "test".into(),
            parts: vec![LayoutPart {
                direction: "horizontal".into(),
                panes: vec![LayoutPane {
                    id: Some("editor".into()),
                    command: "hx README.md".into(),
                    args: vec![],
                    run: true,
                    config: HashMap::new(),
                }],
                ..Default::default()
            }],
            ..Default::default()
        };

        let kdl = layout.to_kdl();
        assert!(kdl.contains("pane split_direction=\"horizontal\""));
        assert!(kdl.contains("pane name=\"editor\" command=\"hx\""));
        assert!(kdl.contains("args \"README.md\""));
        assert!(!kdl.contains("part {"));
        assert!(!kdl.contains("run {"));
    }

    #[test]
    fn borderless_moves_from_containers_onto_leaf_panes() {
        let layout = LayoutConfig {
            name: "test".into(),
            parts: vec![LayoutPart {
                direction: "horizontal".into(),
                borderless: true,
                panes: vec![LayoutPane {
                    id: Some("editor".into()),
                    command: "hx".into(),
                    ..Default::default()
                }],
                parts: vec![LayoutPart {
                    direction: "vertical".into(),
                    panes: vec![LayoutPane {
                        id: Some("git-client".into()),
                        command: "lazygit".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };

        let kdl = layout.to_kdl();
        assert!(kdl.contains("pane name=\"editor\" borderless=true command=\"hx\""));
        assert!(kdl.contains("pane name=\"git-client\" borderless=true command=\"lazygit\""));
        for line in kdl.lines() {
            assert!(
                !(line.contains("borderless=true") && line.trim_end().ends_with('{')),
                "borderless must not be set on a pane with nested panes: {line}"
            );
        }
    }
}
