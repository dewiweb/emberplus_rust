//! Glow root element and container types.

use crate::error::{GlowError, Result};
use crate::ber::{Tag, BerReader, BerWriter, BerValue};
use super::tags;
use super::element::EmberValue;

/// A path through the Ember+ tree (sequence of node numbers).
pub type EmberPath = Vec<i32>;

/// Parse a path string (dot-separated numeric) into a path vector.
///
/// Accepts ONLY numeric paths like "28548.28549.54770".
///
/// **IMPORTANT**: Ember+ identifier strings like "_40001070" are NOT
/// numeric hex conversions of node numbers. On Lawo MC², identifier
/// "_40001070" (EQ) resolves to number 41878, NOT 0x40001070.
/// For identifier-based paths (e.g. "_2/_1/_1/_3e9"), use
/// `EmberClient::navigate_by_identifiers()` instead.
pub fn parse_path(path: &str) -> Result<EmberPath> {
    if path.is_empty() {
        return Ok(vec![]);
    }
    
    path.split('.')
        .map(|s| {
            s.parse::<i32>().map_err(|_| {
                GlowError::InvalidPath(format!(
                    "invalid path component: '{}'. Use navigate_by_identifiers() for identifier paths like _2/_1/_3e9",
                    s
                )).into()
            })
        })
        .collect()
}

/// Format a path vector as a string.
pub fn format_path(path: &EmberPath) -> String {
    path.iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// Glow root element - the top-level container for Ember+ messages.
#[derive(Debug, Clone, Default)]
pub struct GlowRoot {
    /// Root elements (can be a mix of nodes, parameters, etc.)
    pub elements: Vec<GlowElement>,
    /// Invocation results
    pub invocation_results: Vec<InvocationResult>,
    /// Stream entries
    pub streams: Vec<StreamEntry>,
}

impl GlowRoot {
    /// Create an empty root.
    pub fn new() -> Self {
        GlowRoot::default()
    }

    /// Create a root with elements.
    pub fn with_elements(elements: Vec<GlowElement>) -> Self {
        GlowRoot {
            elements,
            ..Default::default()
        }
    }

    /// Create a root with a single command for getting the root directory.
    pub fn get_directory() -> Self {
        GlowRoot {
            elements: vec![GlowElement::Command(GlowCommand::GetDirectory)],
            ..Default::default()
        }
    }

    /// Add an element.
    pub fn add_element(&mut self, element: GlowElement) {
        self.elements.push(element);
    }

    /// Check if this root is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty() 
            && self.invocation_results.is_empty() 
            && self.streams.is_empty()
    }
}

/// A Glow element (can be a node, parameter, function, matrix, or command).
#[derive(Debug, Clone)]
pub enum GlowElement {
    /// A node in the tree
    Node(GlowNode),
    /// A parameter
    Parameter(GlowParameter),
    /// A function
    Function(GlowFunction),
    /// A matrix
    Matrix(GlowMatrix),
    /// A command
    Command(GlowCommand),
    /// A qualified node (with path)
    QualifiedNode(EmberPath, GlowNode),
    /// A qualified parameter (with path)
    QualifiedParameter(EmberPath, GlowParameter),
    /// A qualified function (with path)
    QualifiedFunction(EmberPath, GlowFunction),
    /// A qualified matrix (with path)
    QualifiedMatrix(EmberPath, GlowMatrix),
    /// A template
    Template(GlowTemplate),
}

impl GlowElement {
    /// Get the element number if this is a numbered element.
    pub fn number(&self) -> Option<i32> {
        match self {
            GlowElement::Node(n) => Some(n.number),
            GlowElement::Parameter(p) => Some(p.number),
            GlowElement::Function(f) => Some(f.number),
            GlowElement::Matrix(m) => Some(m.number),
            _ => None,
        }
    }

    /// Get the path if this is a qualified element.
    pub fn path(&self) -> Option<&EmberPath> {
        match self {
            GlowElement::QualifiedNode(path, _) => Some(path),
            GlowElement::QualifiedParameter(path, _) => Some(path),
            GlowElement::QualifiedFunction(path, _) => Some(path),
            GlowElement::QualifiedMatrix(path, _) => Some(path),
            _ => None,
        }
    }

    /// Check if this is a node.
    pub fn is_node(&self) -> bool {
        matches!(self, GlowElement::Node(_) | GlowElement::QualifiedNode(_, _))
    }

    /// Check if this is a parameter.
    pub fn is_parameter(&self) -> bool {
        matches!(self, GlowElement::Parameter(_) | GlowElement::QualifiedParameter(_, _))
    }

    /// Check if this is a function.
    pub fn is_function(&self) -> bool {
        matches!(self, GlowElement::Function(_) | GlowElement::QualifiedFunction(_, _))
    }

    /// Check if this is a matrix.
    pub fn is_matrix(&self) -> bool {
        matches!(self, GlowElement::Matrix(_) | GlowElement::QualifiedMatrix(_, _))
    }

    /// Check if this is a command.
    pub fn is_command(&self) -> bool {
        matches!(self, GlowElement::Command(_))
    }
}

/// A node in the Ember+ tree.
#[derive(Debug, Clone, Default)]
pub struct GlowNode {
    /// Node number (within parent)
    pub number: i32,
    /// Identifier string
    pub identifier: Option<String>,
    /// Description string
    pub description: Option<String>,
    /// Whether this is the root node
    pub is_root: Option<bool>,
    /// Whether this node is online
    pub is_online: Option<bool>,
    /// Schema identifiers
    pub schema_identifiers: Option<String>,
    /// Template reference
    pub template_reference: Option<EmberPath>,
    /// Child elements
    pub children: Vec<GlowElement>,
}

impl GlowNode {
    /// Create a new node with the given number.
    pub fn new(number: i32) -> Self {
        GlowNode {
            number,
            ..Default::default()
        }
    }

    /// Create a node with identifier.
    pub fn with_identifier(number: i32, identifier: &str) -> Self {
        GlowNode {
            number,
            identifier: Some(identifier.to_string()),
            ..Default::default()
        }
    }

    /// Set the description.
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Set the is_root flag.
    pub fn is_root(mut self, is_root: bool) -> Self {
        self.is_root = Some(is_root);
        self
    }

    /// Add a child element.
    pub fn add_child(&mut self, child: GlowElement) {
        self.children.push(child);
    }
}

/// A parameter in the Ember+ tree.
#[derive(Debug, Clone, Default)]
pub struct GlowParameter {
    /// Parameter number (within parent)
    pub number: i32,
    /// Identifier string
    pub identifier: Option<String>,
    /// Description string
    pub description: Option<String>,
    /// Current value
    pub value: Option<EmberValue>,
    /// Minimum value
    pub minimum: Option<EmberValue>,
    /// Maximum value
    pub maximum: Option<EmberValue>,
    /// Access mode
    pub access: Option<super::element::ParameterAccess>,
    /// Format string
    pub format: Option<String>,
    /// Enumeration string (for enum types)
    pub enumeration: Option<String>,
    /// Factor for value conversion
    pub factor: Option<i32>,
    /// Is online
    pub is_online: Option<bool>,
    /// Formula for value transformation
    pub formula: Option<String>,
    /// Step size
    pub step: Option<i32>,
    /// Default value
    pub default: Option<EmberValue>,
    /// Parameter type
    pub parameter_type: Option<super::element::ParameterType>,
    /// Stream identifier
    pub stream_identifier: Option<i32>,
    /// Enumeration map
    pub enum_map: Vec<super::element::StringIntegerPair>,
    /// Stream descriptor
    pub stream_descriptor: Option<super::element::StreamDescriptor>,
    /// Schema identifiers
    pub schema_identifiers: Option<String>,
    /// Template reference
    pub template_reference: Option<EmberPath>,
    /// Child elements
    pub children: Vec<GlowElement>,
}

impl GlowParameter {
    /// Create a new parameter with the given number.
    pub fn new(number: i32) -> Self {
        GlowParameter {
            number,
            ..Default::default()
        }
    }

    /// Create a parameter with identifier and value.
    pub fn with_value(number: i32, identifier: &str, value: EmberValue) -> Self {
        GlowParameter {
            number,
            identifier: Some(identifier.to_string()),
            value: Some(value),
            ..Default::default()
        }
    }

    /// Set the description.
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Set the access mode.
    pub fn access(mut self, access: super::element::ParameterAccess) -> Self {
        self.access = Some(access);
        self
    }

    /// Set the minimum value.
    pub fn minimum(mut self, min: EmberValue) -> Self {
        self.minimum = Some(min);
        self
    }

    /// Set the maximum value.
    pub fn maximum(mut self, max: EmberValue) -> Self {
        self.maximum = Some(max);
        self
    }
}

/// A function in the Ember+ tree.
#[derive(Debug, Clone, Default)]
pub struct GlowFunction {
    /// Function number (within parent)
    pub number: i32,
    /// Identifier string
    pub identifier: Option<String>,
    /// Description string
    pub description: Option<String>,
    /// Arguments description
    pub arguments: Vec<super::element::TupleItemDescription>,
    /// Result description
    pub result: Vec<super::element::TupleItemDescription>,
    /// Template reference
    pub template_reference: Option<EmberPath>,
    /// Child elements
    pub children: Vec<GlowElement>,
}

impl GlowFunction {
    /// Create a new function with the given number.
    pub fn new(number: i32) -> Self {
        GlowFunction {
            number,
            ..Default::default()
        }
    }

    /// Create a function with identifier.
    pub fn with_identifier(number: i32, identifier: &str) -> Self {
        GlowFunction {
            number,
            identifier: Some(identifier.to_string()),
            ..Default::default()
        }
    }
}

/// A matrix in the Ember+ tree.
#[derive(Debug, Clone, Default)]
pub struct GlowMatrix {
    /// Matrix number (within parent)
    pub number: i32,
    /// Identifier string
    pub identifier: Option<String>,
    /// Description string
    pub description: Option<String>,
    /// Matrix type
    pub matrix_type: Option<super::element::MatrixType>,
    /// Addressing mode
    pub addressing_mode: Option<super::element::MatrixAddressingMode>,
    /// Target count
    pub target_count: Option<i32>,
    /// Source count
    pub source_count: Option<i32>,
    /// Maximum connections per target
    pub max_connections_per_target: Option<i32>,
    /// Maximum total connections
    pub max_total_connections: Option<i32>,
    /// Connection parameters location
    pub connection_parameters_location: Option<EmberPath>,
    /// Gain parameter number
    pub gain_parameter_number: Option<i32>,
    /// Labels
    pub labels: Vec<super::element::Label>,
    /// Schema identifiers
    pub schema_identifiers: Option<String>,
    /// Template reference
    pub template_reference: Option<EmberPath>,
    /// Targets
    pub targets: Vec<i32>,
    /// Sources
    pub sources: Vec<i32>,
    /// Connections
    pub connections: Vec<GlowConnection>,
    /// Child elements
    pub children: Vec<GlowElement>,
}

impl GlowMatrix {
    /// Create a new matrix with the given number.
    pub fn new(number: i32) -> Self {
        GlowMatrix {
            number,
            ..Default::default()
        }
    }

    /// Create a matrix with identifier.
    pub fn with_identifier(number: i32, identifier: &str) -> Self {
        GlowMatrix {
            number,
            identifier: Some(identifier.to_string()),
            ..Default::default()
        }
    }
}

/// A matrix connection.
#[derive(Debug, Clone, Default)]
pub struct GlowConnection {
    /// Target number
    pub target: i32,
    /// Source numbers
    pub sources: Vec<i32>,
    /// Operation
    pub operation: Option<super::element::ConnectionOperation>,
    /// Disposition
    pub disposition: Option<super::element::ConnectionDisposition>,
}

impl GlowConnection {
    /// Create a new connection.
    pub fn new(target: i32, sources: Vec<i32>) -> Self {
        GlowConnection {
            target,
            sources,
            ..Default::default()
        }
    }
}

/// A command in the Ember+ protocol.
#[derive(Debug, Clone, PartialEq)]
pub enum GlowCommand {
    /// Subscribe to changes
    Subscribe,
    /// Unsubscribe from changes
    Unsubscribe,
    /// Get directory (children)
    GetDirectory,
    /// Invoke a function
    Invoke {
        /// Invocation ID
        invocation_id: i32,
        /// Arguments
        arguments: Vec<EmberValue>,
    },
}

impl GlowCommand {
    /// Get the command number according to Glow DTD.
    /// See: https://github.com/Lawo/ember-plus/wiki/Ember+-Glow-DTD
    pub fn number(&self) -> i32 {
        match self {
            GlowCommand::Subscribe => 30,      // Was incorrectly 1
            GlowCommand::Unsubscribe => 31,    // Was incorrectly 2
            GlowCommand::GetDirectory => 32,
            GlowCommand::Invoke { .. } => 33,
        }
    }
}

/// A template definition.
#[derive(Debug, Clone, Default)]
pub struct GlowTemplate {
    /// Template number
    pub number: i32,
    /// Element definition
    pub element: Option<Box<GlowElement>>,
    /// Description
    pub description: Option<String>,
}

/// Result of a function invocation.
#[derive(Debug, Clone)]
pub struct InvocationResult {
    /// Invocation ID
    pub invocation_id: i32,
    /// Whether the invocation succeeded
    pub success: bool,
    /// Result values
    pub result: Vec<EmberValue>,
}

impl InvocationResult {
    /// Create a successful result.
    pub fn success(invocation_id: i32, result: Vec<EmberValue>) -> Self {
        InvocationResult {
            invocation_id,
            success: true,
            result,
        }
    }

    /// Create a failed result.
    pub fn failure(invocation_id: i32) -> Self {
        InvocationResult {
            invocation_id,
            success: false,
            result: vec![],
        }
    }
}

/// A stream entry (for streaming parameter values).
#[derive(Debug, Clone)]
pub struct StreamEntry {
    /// Stream identifier
    pub stream_identifier: i32,
    /// Stream value
    pub value: EmberValue,
}

impl StreamEntry {
    /// Create a new stream entry.
    pub fn new(stream_identifier: i32, value: EmberValue) -> Self {
        StreamEntry { stream_identifier, value }
    }
}
