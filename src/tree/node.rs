//! Tree node implementations.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::error::{Error, Result};
use crate::glow::{
    EmberValue, ParameterType, ParameterAccess, MatrixType, MatrixAddressingMode,
    TupleItemDescription, StringIntegerPair, Label,
    EmberPath, GlowElement, GlowRoot, GlowNode, GlowParameter, 
    GlowFunction, GlowMatrix, GlowConnection, parse_path,
};

/// Reference to a tree node.
pub type TreeNodeRef = Arc<RwLock<TreeNode>>;

/// The root of an Ember+ tree.
#[derive(Debug, Default)]
pub struct EmberTree {
    /// Root children (keyed by number)
    children: HashMap<i32, TreeNodeRef>,
    /// Path index for quick lookups
    path_index: HashMap<String, TreeNodeRef>,
}

impl EmberTree {
    /// Create a new empty tree.
    pub fn new() -> Self {
        EmberTree::default()
    }

    /// Add a node at the root level.
    pub fn add_root_child(&mut self, node: TreeNode) -> TreeNodeRef {
        let number = node.number();
        let node_ref = Arc::new(RwLock::new(node));
        self.children.insert(number, Arc::clone(&node_ref));
        self.update_path_index(&node_ref, vec![number]);
        node_ref
    }

    /// Get a root-level child by number.
    pub fn get_child(&self, number: i32) -> Option<TreeNodeRef> {
        self.children.get(&number).cloned()
    }

    /// Get all root-level children.
    pub fn children(&self) -> impl Iterator<Item = &TreeNodeRef> {
        self.children.values()
    }

    /// Get a node by path.
    pub fn get_by_path(&self, path: &[i32]) -> Option<TreeNodeRef> {
        if path.is_empty() {
            return None;
        }

        let path_str = path.iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(".");
        
        if let Some(node) = self.path_index.get(&path_str) {
            return Some(Arc::clone(node));
        }

        // Manual traversal if not in index
        let mut current = self.get_child(path[0])?;
        
        for &number in &path[1..] {
            let node = current.read();
            let child = node.get_child(number)?;
            drop(node);
            current = child;
        }
        
        Some(current)
    }

    /// Get a node by path string (e.g., "0.1.2").
    pub fn get_by_path_str(&self, path: &str) -> Result<Option<TreeNodeRef>> {
        let path = parse_path(path)?;
        Ok(self.get_by_path(&path))
    }

    /// Update the tree from a Glow root message.
    pub fn update_from_glow(&mut self, root: &GlowRoot) {
        for element in &root.elements {
            self.update_element(element, vec![]);
        }
    }

    /// Update a single element in the tree.
    fn update_element(&mut self, element: &GlowElement, parent_path: Vec<i32>) {
        match element {
            GlowElement::Node(node) => {
                let mut path = parent_path.clone();
                path.push(node.number);
                
                let tree_node = self.ensure_node(&path);
                {
                    let mut n = tree_node.write();
                    n.update_from_glow_node(node);
                }
                
                for child in &node.children {
                    self.update_element(child, path.clone());
                }
            }
            GlowElement::Parameter(param) => {
                let mut path = parent_path.clone();
                path.push(param.number);
                
                let tree_node = self.ensure_node(&path);
                {
                    let mut n = tree_node.write();
                    n.update_from_glow_parameter(param);
                }
            }
            GlowElement::Function(func) => {
                let mut path = parent_path.clone();
                path.push(func.number);
                
                let tree_node = self.ensure_node(&path);
                {
                    let mut n = tree_node.write();
                    n.update_from_glow_function(func);
                }
            }
            GlowElement::Matrix(matrix) => {
                let mut path = parent_path.clone();
                path.push(matrix.number);
                
                let tree_node = self.ensure_node(&path);
                {
                    let mut n = tree_node.write();
                    n.update_from_glow_matrix(matrix);
                }
            }
            GlowElement::QualifiedNode(path, node) => {
                let tree_node = self.ensure_node(path);
                {
                    let mut n = tree_node.write();
                    n.update_from_glow_node(node);
                }
                
                for child in &node.children {
                    self.update_element(child, path.clone());
                }
            }
            GlowElement::QualifiedParameter(path, param) => {
                let tree_node = self.ensure_node(path);
                {
                    let mut n = tree_node.write();
                    n.update_from_glow_parameter(param);
                }
            }
            GlowElement::QualifiedFunction(path, func) => {
                let tree_node = self.ensure_node(path);
                {
                    let mut n = tree_node.write();
                    n.update_from_glow_function(func);
                }
            }
            GlowElement::QualifiedMatrix(path, matrix) => {
                let tree_node = self.ensure_node(path);
                {
                    let mut n = tree_node.write();
                    n.update_from_glow_matrix(matrix);
                }
            }
            _ => {}
        }
    }

    /// Ensure a node exists at the given path, creating intermediates as needed.
    fn ensure_node(&mut self, path: &[i32]) -> TreeNodeRef {
        if path.is_empty() {
            panic!("Cannot ensure node at empty path");
        }

        // Check if already exists
        if let Some(node) = self.get_by_path(path) {
            return node;
        }

        // Create root node if needed
        if !self.children.contains_key(&path[0]) {
            let new_node = Arc::new(RwLock::new(TreeNode::new_node(path[0])));
            self.children.insert(path[0], Arc::clone(&new_node));
            let path_str = path[0].to_string();
            self.path_index.insert(path_str, new_node);
        }
        
        let root_node = Arc::clone(self.children.get(&path[0]).unwrap());

        if path.len() == 1 {
            return root_node;
        }

        // Traverse and create intermediate nodes
        let mut current = root_node;
        let mut current_path = vec![path[0]];
        
        for &number in &path[1..] {
            current_path.push(number);
            let child = {
                let mut node = current.write();
                if let Some(existing) = node.get_child(number) {
                    existing
                } else {
                    let new_child = Arc::new(RwLock::new(TreeNode::new_node(number)));
                    node.add_child(Arc::clone(&new_child));
                    new_child
                }
            };
            let path_str = current_path.iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(".");
            self.path_index.insert(path_str, Arc::clone(&child));
            current = child;
        }

        current
    }

    /// Update the path index for a node.
    fn update_path_index(&mut self, node: &TreeNodeRef, path: Vec<i32>) {
        let path_str = path.iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(".");
        self.path_index.insert(path_str, Arc::clone(node));
    }

    /// Clear the tree.
    pub fn clear(&mut self) {
        self.children.clear();
        self.path_index.clear();
    }

    /// Convert the tree to Glow elements.
    pub fn to_glow_elements(&self) -> Vec<GlowElement> {
        self.children.values()
            .map(|node| node.read().to_glow_element())
            .collect()
    }
}

/// Contents of a tree node.
#[derive(Debug, Clone)]
pub enum NodeContents {
    /// A container node
    Node {
        identifier: Option<String>,
        description: Option<String>,
        is_root: Option<bool>,
        is_online: Option<bool>,
        schema_identifiers: Option<String>,
    },
    /// A parameter
    Parameter {
        identifier: Option<String>,
        description: Option<String>,
        value: Option<EmberValue>,
        minimum: Option<EmberValue>,
        maximum: Option<EmberValue>,
        access: Option<ParameterAccess>,
        format: Option<String>,
        enumeration: Option<String>,
        factor: Option<i32>,
        is_online: Option<bool>,
        formula: Option<String>,
        step: Option<i32>,
        default: Option<EmberValue>,
        parameter_type: Option<ParameterType>,
        stream_identifier: Option<i32>,
        enum_map: Vec<StringIntegerPair>,
    },
    /// A function
    Function {
        identifier: Option<String>,
        description: Option<String>,
        arguments: Vec<TupleItemDescription>,
        result: Vec<TupleItemDescription>,
    },
    /// A matrix
    Matrix {
        identifier: Option<String>,
        description: Option<String>,
        matrix_type: Option<MatrixType>,
        addressing_mode: Option<MatrixAddressingMode>,
        target_count: Option<i32>,
        source_count: Option<i32>,
        max_connections_per_target: Option<i32>,
        max_total_connections: Option<i32>,
        targets: Vec<i32>,
        sources: Vec<i32>,
        connections: Vec<GlowConnection>,
        labels: Vec<Label>,
    },
}

impl Default for NodeContents {
    fn default() -> Self {
        NodeContents::Node {
            identifier: None,
            description: None,
            is_root: None,
            is_online: None,
            schema_identifiers: None,
        }
    }
}

/// A node in the Ember+ tree.
#[derive(Debug)]
pub struct TreeNode {
    /// Node number
    number: i32,
    /// Node contents
    contents: NodeContents,
    /// Child nodes
    children: HashMap<i32, TreeNodeRef>,
}

impl TreeNode {
    /// Create a new container node.
    pub fn new_node(number: i32) -> Self {
        TreeNode {
            number,
            contents: NodeContents::Node {
                identifier: None,
                description: None,
                is_root: None,
                is_online: None,
                schema_identifiers: None,
            },
            children: HashMap::new(),
        }
    }

    /// Create a new parameter node.
    pub fn new_parameter(number: i32, value: EmberValue) -> Self {
        TreeNode {
            number,
            contents: NodeContents::Parameter {
                identifier: None,
                description: None,
                value: Some(value),
                minimum: None,
                maximum: None,
                access: Some(ParameterAccess::ReadWrite),
                format: None,
                enumeration: None,
                factor: None,
                is_online: None,
                formula: None,
                step: None,
                default: None,
                parameter_type: None,
                stream_identifier: None,
                enum_map: vec![],
            },
            children: HashMap::new(),
        }
    }

    /// Get the node number.
    pub fn number(&self) -> i32 {
        self.number
    }

    /// Get the node contents.
    pub fn contents(&self) -> &NodeContents {
        &self.contents
    }

    /// Get mutable node contents.
    pub fn contents_mut(&mut self) -> &mut NodeContents {
        &mut self.contents
    }

    /// Get the identifier.
    pub fn identifier(&self) -> Option<&str> {
        match &self.contents {
            NodeContents::Node { identifier, .. } => identifier.as_deref(),
            NodeContents::Parameter { identifier, .. } => identifier.as_deref(),
            NodeContents::Function { identifier, .. } => identifier.as_deref(),
            NodeContents::Matrix { identifier, .. } => identifier.as_deref(),
        }
    }

    /// Get the description.
    pub fn description(&self) -> Option<&str> {
        match &self.contents {
            NodeContents::Node { description, .. } => description.as_deref(),
            NodeContents::Parameter { description, .. } => description.as_deref(),
            NodeContents::Function { description, .. } => description.as_deref(),
            NodeContents::Matrix { description, .. } => description.as_deref(),
        }
    }

    /// Check if this is a container node.
    pub fn is_node(&self) -> bool {
        matches!(self.contents, NodeContents::Node { .. })
    }

    /// Check if this is a parameter.
    pub fn is_parameter(&self) -> bool {
        matches!(self.contents, NodeContents::Parameter { .. })
    }

    /// Check if this is a function.
    pub fn is_function(&self) -> bool {
        matches!(self.contents, NodeContents::Function { .. })
    }

    /// Check if this is a matrix.
    pub fn is_matrix(&self) -> bool {
        matches!(self.contents, NodeContents::Matrix { .. })
    }

    /// Get the parameter value if this is a parameter.
    pub fn value(&self) -> Option<&EmberValue> {
        match &self.contents {
            NodeContents::Parameter { value, .. } => value.as_ref(),
            _ => None,
        }
    }

    /// Set the parameter value.
    pub fn set_value(&mut self, new_value: EmberValue) {
        if let NodeContents::Parameter { value, .. } = &mut self.contents {
            *value = Some(new_value);
        }
    }

    /// Add a child node.
    pub fn add_child(&mut self, child: TreeNodeRef) {
        let number = child.read().number;
        self.children.insert(number, child);
    }

    /// Get a child by number.
    pub fn get_child(&self, number: i32) -> Option<TreeNodeRef> {
        self.children.get(&number).cloned()
    }

    /// Get all children.
    pub fn children(&self) -> impl Iterator<Item = &TreeNodeRef> {
        self.children.values()
    }

    /// Update from a Glow node.
    pub fn update_from_glow_node(&mut self, glow: &GlowNode) {
        self.contents = NodeContents::Node {
            identifier: glow.identifier.clone(),
            description: glow.description.clone(),
            is_root: glow.is_root,
            is_online: glow.is_online,
            schema_identifiers: glow.schema_identifiers.clone(),
        };
    }

    /// Update from a Glow parameter.
    pub fn update_from_glow_parameter(&mut self, glow: &GlowParameter) {
        self.contents = NodeContents::Parameter {
            identifier: glow.identifier.clone(),
            description: glow.description.clone(),
            value: glow.value.clone(),
            minimum: glow.minimum.clone(),
            maximum: glow.maximum.clone(),
            access: glow.access,
            format: glow.format.clone(),
            enumeration: glow.enumeration.clone(),
            factor: glow.factor,
            is_online: glow.is_online,
            formula: glow.formula.clone(),
            step: glow.step,
            default: glow.default.clone(),
            parameter_type: glow.parameter_type,
            stream_identifier: glow.stream_identifier,
            enum_map: glow.enum_map.clone(),
        };
    }

    /// Update from a Glow function.
    pub fn update_from_glow_function(&mut self, glow: &GlowFunction) {
        self.contents = NodeContents::Function {
            identifier: glow.identifier.clone(),
            description: glow.description.clone(),
            arguments: glow.arguments.clone(),
            result: glow.result.clone(),
        };
    }

    /// Update from a Glow matrix.
    pub fn update_from_glow_matrix(&mut self, glow: &GlowMatrix) {
        self.contents = NodeContents::Matrix {
            identifier: glow.identifier.clone(),
            description: glow.description.clone(),
            matrix_type: glow.matrix_type,
            addressing_mode: glow.addressing_mode,
            target_count: glow.target_count,
            source_count: glow.source_count,
            max_connections_per_target: glow.max_connections_per_target,
            max_total_connections: glow.max_total_connections,
            targets: glow.targets.clone(),
            sources: glow.sources.clone(),
            connections: glow.connections.clone(),
            labels: glow.labels.clone(),
        };
    }

    /// Convert to a Glow element.
    pub fn to_glow_element(&self) -> GlowElement {
        match &self.contents {
            NodeContents::Node { identifier, description, is_root, is_online, schema_identifiers } => {
                let mut node = GlowNode::new(self.number);
                node.identifier = identifier.clone();
                node.description = description.clone();
                node.is_root = *is_root;
                node.is_online = *is_online;
                node.schema_identifiers = schema_identifiers.clone();
                node.children = self.children.values()
                    .map(|c| c.read().to_glow_element())
                    .collect();
                GlowElement::Node(node)
            }
            NodeContents::Parameter { identifier, description, value, minimum, maximum, access, format, enumeration, factor, is_online, formula, step, default, parameter_type, stream_identifier, enum_map } => {
                let mut param = GlowParameter::new(self.number);
                param.identifier = identifier.clone();
                param.description = description.clone();
                param.value = value.clone();
                param.minimum = minimum.clone();
                param.maximum = maximum.clone();
                param.access = *access;
                param.format = format.clone();
                param.enumeration = enumeration.clone();
                param.factor = *factor;
                param.is_online = *is_online;
                param.formula = formula.clone();
                param.step = *step;
                param.default = default.clone();
                param.parameter_type = *parameter_type;
                param.stream_identifier = *stream_identifier;
                param.enum_map = enum_map.clone();
                GlowElement::Parameter(param)
            }
            NodeContents::Function { identifier, description, arguments, result } => {
                let mut func = GlowFunction::new(self.number);
                func.identifier = identifier.clone();
                func.description = description.clone();
                func.arguments = arguments.clone();
                func.result = result.clone();
                GlowElement::Function(func)
            }
            NodeContents::Matrix { identifier, description, matrix_type, addressing_mode, target_count, source_count, max_connections_per_target, max_total_connections, targets, sources, connections, labels } => {
                let mut matrix = GlowMatrix::new(self.number);
                matrix.identifier = identifier.clone();
                matrix.description = description.clone();
                matrix.matrix_type = *matrix_type;
                matrix.addressing_mode = *addressing_mode;
                matrix.target_count = *target_count;
                matrix.source_count = *source_count;
                matrix.max_connections_per_target = *max_connections_per_target;
                matrix.max_total_connections = *max_total_connections;
                matrix.targets = targets.clone();
                matrix.sources = sources.clone();
                matrix.connections = connections.clone();
                matrix.labels = labels.clone();
                GlowElement::Matrix(matrix)
            }
        }
    }
}
