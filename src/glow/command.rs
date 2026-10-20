//! Glow command handling.

use crate::error::Result;
use super::root::{GlowCommand, GlowElement, EmberPath};
use super::element::EmberValue;

/// Direction field mask for GetDirectory command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DirFieldMask {
    /// Include identifier
    pub identifier: bool,
    /// Include description
    pub description: bool,
    /// Include tree (children)
    pub tree: bool,
    /// Include value
    pub value: bool,
    /// Include connections (for matrices)
    pub connections: bool,
}

impl DirFieldMask {
    /// Create a mask requesting all fields.
    pub fn all() -> Self {
        DirFieldMask {
            identifier: true,
            description: true,
            tree: true,
            value: true,
            connections: true,
        }
    }

    /// Create a mask requesting default fields.
    pub fn default_fields() -> Self {
        DirFieldMask {
            identifier: true,
            description: true,
            tree: false,
            value: true,
            connections: false,
        }
    }

    /// Create from a bitmask integer.
    pub fn from_bits(bits: i32) -> Self {
        DirFieldMask {
            identifier: (bits & 0x01) != 0,
            description: (bits & 0x02) != 0,
            tree: (bits & 0x04) != 0,
            value: (bits & 0x08) != 0,
            connections: (bits & 0x10) != 0,
        }
    }

    /// Convert to a bitmask integer.
    pub fn to_bits(&self) -> i32 {
        let mut bits = 0;
        if self.identifier { bits |= 0x01; }
        if self.description { bits |= 0x02; }
        if self.tree { bits |= 0x04; }
        if self.value { bits |= 0x08; }
        if self.connections { bits |= 0x10; }
        bits
    }
}

/// Command builder for creating Glow commands.
pub struct CommandBuilder;

impl CommandBuilder {
    /// Create a GetDirectory command for the root.
    pub fn get_root_directory() -> GlowElement {
        GlowElement::Command(GlowCommand::GetDirectory)
    }

    /// Create a Subscribe command.
    pub fn subscribe() -> GlowElement {
        GlowElement::Command(GlowCommand::Subscribe)
    }

    /// Create an Unsubscribe command.
    pub fn unsubscribe() -> GlowElement {
        GlowElement::Command(GlowCommand::Unsubscribe)
    }

    /// Create an Invoke command.
    pub fn invoke(invocation_id: i32, arguments: Vec<EmberValue>) -> GlowElement {
        GlowElement::Command(GlowCommand::Invoke {
            invocation_id,
            arguments,
        })
    }

    /// Create a command to get directory at a specific path.
    pub fn get_directory_at_path(path: &EmberPath) -> Vec<GlowElement> {
        Self::wrap_command_at_path(path, GlowCommand::GetDirectory, None)
    }

    /// Create a command to get directory at a specific path with node info.
    pub fn get_directory_at_path_with_info(path: &EmberPath, identifier: Option<&str>) -> Vec<GlowElement> {
        Self::wrap_command_at_path(path, GlowCommand::GetDirectory, identifier)
    }

    /// Create a command to subscribe at a specific path.
    pub fn subscribe_at_path(path: &EmberPath) -> Vec<GlowElement> {
        Self::wrap_command_at_path(path, GlowCommand::Subscribe, None)
    }

    /// Create a command to unsubscribe at a specific path.
    pub fn unsubscribe_at_path(path: &EmberPath) -> Vec<GlowElement> {
        Self::wrap_command_at_path(path, GlowCommand::Unsubscribe, None)
    }

    /// Create subscribe commands for multiple paths in a single message.
    /// All QualifiedNodes are combined into one Vec for a single GlowRoot packet.
    pub fn subscribe_many_paths(paths: &[EmberPath]) -> Vec<GlowElement> {
        paths.iter()
            .flat_map(|path| Self::wrap_command_at_path(path, GlowCommand::Subscribe, None))
            .collect()
    }

    /// Create unsubscribe commands for multiple paths in a single message.
    pub fn unsubscribe_many_paths(paths: &[EmberPath]) -> Vec<GlowElement> {
        paths.iter()
            .flat_map(|path| Self::wrap_command_at_path(path, GlowCommand::Unsubscribe, None))
            .collect()
    }

    /// Wrap a command in a QualifiedNode to reach the given path.
    fn wrap_command_at_path(path: &EmberPath, command: GlowCommand, identifier: Option<&str>) -> Vec<GlowElement> {
        if path.is_empty() {
            return vec![GlowElement::Command(command)];
        }

        // Use QualifiedNode with the path and command as child
        let mut node = super::root::GlowNode::new(0);
        node.identifier = identifier.map(|s| s.to_string());
        node.children.push(GlowElement::Command(command));
        vec![GlowElement::QualifiedNode(path.clone(), node)]
    }

    /// Create a setValue command for a parameter at a path.
    pub fn set_value_at_path(path: &EmberPath, value: EmberValue) -> Vec<GlowElement> {
        if path.is_empty() {
            return vec![];
        }

        let param_number = *path.last().unwrap();
        let parent_path = &path[..path.len() - 1];

        let mut param = super::root::GlowParameter::new(param_number);
        param.value = Some(value);
        let param_element = GlowElement::Parameter(param);

        if parent_path.is_empty() {
            return vec![param_element];
        }

        // Build from innermost to outermost
        let mut current = param_element;

        for &number in parent_path.iter().rev() {
            let mut node = super::root::GlowNode::new(number);
            node.children.push(current);
            current = GlowElement::Node(node);
        }

        vec![current]
    }

    /// Create an invoke command for a function at a path.
    pub fn invoke_at_path(
        path: &EmberPath,
        invocation_id: i32,
        arguments: Vec<EmberValue>,
    ) -> Vec<GlowElement> {
        if path.is_empty() {
            return vec![];
        }

        // Use QualifiedFunction with the full path
        let mut func = super::root::GlowFunction::new(0);
        func.children.push(GlowElement::Command(GlowCommand::Invoke {
            invocation_id,
            arguments,
        }));
        vec![GlowElement::QualifiedFunction(path.clone(), func)]
    }

    /// Create a matrix connection operation.
    pub fn matrix_connect(
        path: &EmberPath,
        target: i32,
        sources: Vec<i32>,
        operation: super::element::ConnectionOperation,
    ) -> Vec<GlowElement> {
        if path.is_empty() {
            return vec![];
        }

        let matrix_number = *path.last().unwrap();
        let parent_path = &path[..path.len() - 1];

        let connection = super::root::GlowConnection {
            target,
            sources,
            operation: Some(operation),
            disposition: None,
        };

        let mut matrix = super::root::GlowMatrix::new(matrix_number);
        matrix.connections.push(connection);
        let matrix_element = GlowElement::Matrix(matrix);

        if parent_path.is_empty() {
            return vec![matrix_element];
        }

        // Build from innermost to outermost
        let mut current = matrix_element;

        for &number in parent_path.iter().rev() {
            let mut node = super::root::GlowNode::new(number);
            node.children.push(current);
            current = GlowElement::Node(node);
        }

        vec![current]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirfield_mask() {
        let mask = DirFieldMask::all();
        let bits = mask.to_bits();
        let decoded = DirFieldMask::from_bits(bits);
        
        assert_eq!(mask.identifier, decoded.identifier);
        assert_eq!(mask.description, decoded.description);
        assert_eq!(mask.tree, decoded.tree);
        assert_eq!(mask.value, decoded.value);
        assert_eq!(mask.connections, decoded.connections);
    }

    #[test]
    fn test_wrap_command_at_path() {
        let path = vec![0, 1, 2];
        let elements = CommandBuilder::get_directory_at_path(&path);

        assert_eq!(elements.len(), 1);

        // Should be a QualifiedNode carrying the full path, with the command as child
        if let GlowElement::QualifiedNode(p, node) = &elements[0] {
            assert_eq!(p, &path);
            assert!(matches!(node.children[0], GlowElement::Command(GlowCommand::GetDirectory)));
        } else {
            panic!("Expected QualifiedNode");
        }
    }
}
