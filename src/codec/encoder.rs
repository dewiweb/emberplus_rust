//! Glow encoder - converts Glow structures to BER bytes.

use crate::error::Result;
use crate::ber::{BerWriter, Tag, TagClass, TagType};
use crate::glow::{
    GlowRoot, GlowElement, GlowNode, GlowParameter, GlowFunction,
    GlowMatrix, GlowConnection, GlowCommand, GlowTemplate,
    InvocationResult, StreamEntry, EmberPath, EmberValue,
};

/// Encoder for Glow messages.
pub struct GlowEncoder;

impl GlowEncoder {
    /// Encode a GlowRoot to BER bytes.
    pub fn encode(root: &GlowRoot) -> Result<Vec<u8>> {
        let mut writer = BerWriter::new();
        Self::encode_root(&mut writer, root)?;
        Ok(writer.into_bytes())
    }

    /// Encode the root element.
    fn encode_root(writer: &mut BerWriter, root: &GlowRoot) -> Result<()> {
        writer.write_application(0, |w| {
            // Encode root element collection
            if !root.elements.is_empty() {
                w.write_application(11, |w2| {
                    for element in &root.elements {
                        Self::encode_element(w2, element)?;
                    }
                    Ok(())
                })?;
            }

            // Encode invocation results
            for result in &root.invocation_results {
                Self::encode_invocation_result(w, result)?;
            }

            // Encode stream entries
            if !root.streams.is_empty() {
                w.write_application(6, |w2| {
                    for entry in &root.streams {
                        Self::encode_stream_entry(w2, entry)?;
                    }
                    Ok(())
                })?;
            }

            Ok(())
        })
    }

    /// Encode an element.
    fn encode_element(writer: &mut BerWriter, element: &GlowElement) -> Result<()> {
        match element {
            GlowElement::Node(node) => Self::encode_node(writer, node),
            GlowElement::Parameter(param) => Self::encode_parameter(writer, param),
            GlowElement::Function(func) => Self::encode_function(writer, func),
            GlowElement::Matrix(matrix) => Self::encode_matrix(writer, matrix),
            GlowElement::Command(cmd) => Self::encode_numbered_command(writer, cmd),
            GlowElement::QualifiedNode(path, node) => Self::encode_qualified_node(writer, path, node),
            GlowElement::QualifiedParameter(path, param) => Self::encode_qualified_parameter(writer, path, param),
            GlowElement::QualifiedFunction(path, func) => Self::encode_qualified_function(writer, path, func),
            GlowElement::QualifiedMatrix(path, matrix) => Self::encode_qualified_matrix(writer, path, matrix),
            GlowElement::Template(template) => Self::encode_template(writer, template),
        }
    }

    /// Encode a node.
    fn encode_node(writer: &mut BerWriter, node: &GlowNode) -> Result<()> {
        writer.write_application(3, |w| {
            // Number [0]
            w.write_context_integer(0, node.number as i64)?;

            // Contents [1]
            if node.identifier.is_some() || node.description.is_some() 
                || node.is_root.is_some() || node.is_online.is_some() {
                w.write_context(1, |w2| {
                    Self::encode_node_contents(w2, node)
                })?;
            }

            // Children [2]
            if !node.children.is_empty() {
                w.write_context(2, |w2| {
                    w2.write_application(4, |w3| {
                        for child in &node.children {
                            Self::encode_element(w3, child)?;
                        }
                        Ok(())
                    })
                })?;
            }

            Ok(())
        })
    }

    /// Encode node contents wrapped in a SET.
    fn encode_node_contents(writer: &mut BerWriter, node: &GlowNode) -> Result<()> {
        // Node contents must be wrapped in a SET (0x31)
        writer.write_set(|w| {
            if let Some(id) = &node.identifier {
                Self::encode_context_utf8string(w, 0, id)?;
            }
            if let Some(desc) = &node.description {
                Self::encode_context_utf8string(w, 1, desc)?;
            }
            if let Some(is_root) = node.is_root {
                Self::encode_context_bool(w, 2, is_root)?;
            }
            if let Some(is_online) = node.is_online {
                Self::encode_context_bool(w, 3, is_online)?;
            }
            if let Some(schema) = &node.schema_identifiers {
                Self::encode_context_utf8string(w, 4, schema)?;
            }
            Ok(())
        })
    }

    /// Encode a parameter.
    fn encode_parameter(writer: &mut BerWriter, param: &GlowParameter) -> Result<()> {
        writer.write_application(1, |w| {
            // Number [0]
            w.write_context_integer(0, param.number as i64)?;

            // Contents [1]
            w.write_context(1, |w2| {
                Self::encode_parameter_contents(w2, param)
            })?;

            // Children [2]
            if !param.children.is_empty() {
                w.write_context(2, |w2| {
                    w2.write_application(4, |w3| {
                        for child in &param.children {
                            Self::encode_element(w3, child)?;
                        }
                        Ok(())
                    })
                })?;
            }

            Ok(())
        })
    }

    /// Encode parameter contents.
    fn encode_parameter_contents(writer: &mut BerWriter, param: &GlowParameter) -> Result<()> {
        if let Some(id) = &param.identifier {
            Self::encode_context_string(writer, 0, id)?;
        }
        if let Some(desc) = &param.description {
            Self::encode_context_string(writer, 1, desc)?;
        }
        if let Some(value) = &param.value {
            Self::encode_context_value(writer, 2, value)?;
        }
        if let Some(min) = &param.minimum {
            Self::encode_context_value(writer, 3, min)?;
        }
        if let Some(max) = &param.maximum {
            Self::encode_context_value(writer, 4, max)?;
        }
        if let Some(access) = param.access {
            writer.write_context_integer(5, access as i64)?;
        }
        if let Some(format) = &param.format {
            Self::encode_context_string(writer, 6, format)?;
        }
        if let Some(enumeration) = &param.enumeration {
            Self::encode_context_string(writer, 7, enumeration)?;
        }
        if let Some(factor) = param.factor {
            writer.write_context_integer(8, factor as i64)?;
        }
        if let Some(is_online) = param.is_online {
            Self::encode_context_bool(writer, 9, is_online)?;
        }
        if let Some(formula) = &param.formula {
            Self::encode_context_string(writer, 10, formula)?;
        }
        if let Some(step) = param.step {
            writer.write_context_integer(11, step as i64)?;
        }
        if let Some(default) = &param.default {
            Self::encode_context_value(writer, 12, default)?;
        }
        if let Some(param_type) = param.parameter_type {
            writer.write_context_integer(13, param_type as i64)?;
        }
        if let Some(stream_id) = param.stream_identifier {
            writer.write_context_integer(14, stream_id as i64)?;
        }
        Ok(())
    }

    /// Encode a function.
    fn encode_function(writer: &mut BerWriter, func: &GlowFunction) -> Result<()> {
        writer.write_application(19, |w| {
            w.write_context_integer(0, func.number as i64)?;

            w.write_context(1, |w2| {
                if let Some(id) = &func.identifier {
                    Self::encode_context_string(w2, 0, id)?;
                }
                if let Some(desc) = &func.description {
                    Self::encode_context_string(w2, 1, desc)?;
                }
                // Arguments and result tuples would go here
                Ok(())
            })?;

            if !func.children.is_empty() {
                w.write_context(2, |w2| {
                    w2.write_application(4, |w3| {
                        for child in &func.children {
                            Self::encode_element(w3, child)?;
                        }
                        Ok(())
                    })
                })?;
            }

            Ok(())
        })
    }

    /// Encode a matrix.
    fn encode_matrix(writer: &mut BerWriter, matrix: &GlowMatrix) -> Result<()> {
        writer.write_application(13, |w| {
            w.write_context_integer(0, matrix.number as i64)?;

            w.write_context(1, |w2| {
                if let Some(id) = &matrix.identifier {
                    Self::encode_context_string(w2, 0, id)?;
                }
                if let Some(desc) = &matrix.description {
                    Self::encode_context_string(w2, 1, desc)?;
                }
                if let Some(mt) = matrix.matrix_type {
                    w2.write_context_integer(2, mt as i64)?;
                }
                if let Some(am) = matrix.addressing_mode {
                    w2.write_context_integer(3, am as i64)?;
                }
                if let Some(tc) = matrix.target_count {
                    w2.write_context_integer(4, tc as i64)?;
                }
                if let Some(sc) = matrix.source_count {
                    w2.write_context_integer(5, sc as i64)?;
                }
                Ok(())
            })?;

            // Encode connections if present
            if !matrix.connections.is_empty() {
                w.write_context(5, |w2| {
                    for conn in &matrix.connections {
                        Self::encode_connection(w2, conn)?;
                    }
                    Ok(())
                })?;
            }

            Ok(())
        })
    }

    /// Encode a connection.
    fn encode_connection(writer: &mut BerWriter, conn: &GlowConnection) -> Result<()> {
        writer.write_application(16, |w| {
            w.write_context_integer(0, conn.target as i64)?;

            if !conn.sources.is_empty() {
                w.write_context(1, |w2| {
                    for &src in &conn.sources {
                        w2.write_integer(src as i64)?;
                    }
                    Ok(())
                })?;
            }

            if let Some(op) = conn.operation {
                w.write_context_integer(2, op as i64)?;
            }

            if let Some(disp) = conn.disposition {
                w.write_context_integer(3, disp as i64)?;
            }

            Ok(())
        })
    }

    /// Encode a command wrapped in a NumberedTreeNode (context [0]).
    /// This is required when the command is at root level.
    fn encode_numbered_command(writer: &mut BerWriter, cmd: &GlowCommand) -> Result<()> {
        // Wrap in context [0] for NumberedTreeNode at root
        writer.write_context(0, |w| {
            Self::encode_command(w, cmd)
        })
    }

    /// Encode a command (APPLICATION 2).
    fn encode_command(writer: &mut BerWriter, cmd: &GlowCommand) -> Result<()> {
        writer.write_application(2, |w| {
            // Command number in context [0] with universal INTEGER inside
            w.write_context(0, |w2| {
                w2.write_integer(cmd.number() as i64)
            })?;

            match cmd {
                GlowCommand::GetDirectory => {
                    // DirFieldMask [1]: FieldFlags::All = -1
                    w.write_context(1, |w2| {
                        w2.write_integer(-1)
                    })?;
                }
                GlowCommand::Invoke { invocation_id, arguments } => {
                    // Invoke options in context[2] with APPLICATION 22 (Invocation)
                    w.write_context(2, |w2| {
                        w2.write_application(22, |w3| {
                            // Invocation ID in context[0]
                            w3.write_context(0, |w4| {
                                w4.write_integer(*invocation_id as i64)
                            })?;
                            // Arguments in context[1] as SEQUENCE
                            w3.write_context(1, |w4| {
                                w4.write_sequence(|w5| {
                                    for arg in arguments {
                                        Self::encode_value(w5, arg)?;
                                    }
                                    Ok(())
                                })
                            })?;
                            Ok(())
                        })
                    })?;
                }
                _ => {}
            }

            Ok(())
        })
    }

    /// Encode a qualified node.
    fn encode_qualified_node(writer: &mut BerWriter, path: &EmberPath, node: &GlowNode) -> Result<()> {
        // Wrap in context [0] for NumberedTreeNode
        writer.write_context(0, |w| {
            w.write_application(10, |w2| {
                Self::encode_context_path(w2, 0, path)?;

                // Only include context[1] if there are contents to encode
                if node.identifier.is_some() || node.description.is_some()
                    || node.is_root.is_some() || node.is_online.is_some()
                    || node.schema_identifiers.is_some() {
                    w2.write_context(1, |w3| {
                        Self::encode_node_contents(w3, node)
                    })?;
                }

                if !node.children.is_empty() {
                    w2.write_context(2, |w3| {
                        w3.write_application(4, |w4| {
                            for child in &node.children {
                                Self::encode_element(w4, child)?;
                            }
                            Ok(())
                        })
                    })?;
                }

                Ok(())
            })
        })
    }

    /// Encode a qualified parameter.
    fn encode_qualified_parameter(writer: &mut BerWriter, path: &EmberPath, param: &GlowParameter) -> Result<()> {
        writer.write_application(9, |w| {
            Self::encode_context_path(w, 0, path)?;

            w.write_context(1, |w2| {
                Self::encode_parameter_contents(w2, param)
            })?;

            Ok(())
        })
    }

    /// Encode a qualified function.
    fn encode_qualified_function(writer: &mut BerWriter, path: &EmberPath, func: &GlowFunction) -> Result<()> {
        // Wrap in context [0] for NumberedTreeNode
        writer.write_context(0, |w| {
            w.write_application(20, |w2| {
                Self::encode_context_path(w2, 0, path)?;

                // Only write contents if there's something to write
                if func.identifier.is_some() || func.description.is_some() {
                    w2.write_context(1, |w3| {
                        if let Some(id) = &func.identifier {
                            Self::encode_context_string(w3, 0, id)?;
                        }
                        if let Some(desc) = &func.description {
                            Self::encode_context_string(w3, 1, desc)?;
                        }
                        Ok(())
                    })?;
                }

                // Encode children (commands) in context [2]
                if !func.children.is_empty() {
                    w2.write_context(2, |w3| {
                        w3.write_application(4, |w4| {
                            for child in &func.children {
                                Self::encode_element(w4, child)?;
                            }
                            Ok(())
                        })
                    })?;
                }

                Ok(())
            })
        })
    }

    /// Encode a qualified matrix.
    fn encode_qualified_matrix(writer: &mut BerWriter, path: &EmberPath, matrix: &GlowMatrix) -> Result<()> {
        writer.write_application(17, |w| {
            Self::encode_context_path(w, 0, path)?;

            w.write_context(1, |w2| {
                if let Some(id) = &matrix.identifier {
                    Self::encode_context_string(w2, 0, id)?;
                }
                if let Some(desc) = &matrix.description {
                    Self::encode_context_string(w2, 1, desc)?;
                }
                Ok(())
            })?;

            Ok(())
        })
    }

    /// Encode a template.
    fn encode_template(writer: &mut BerWriter, template: &GlowTemplate) -> Result<()> {
        writer.write_application(24, |w| {
            w.write_context_integer(0, template.number as i64)?;
            if let Some(desc) = &template.description {
                Self::encode_context_string(w, 2, desc)?;
            }
            Ok(())
        })
    }

    /// Encode an invocation result.
    fn encode_invocation_result(writer: &mut BerWriter, result: &InvocationResult) -> Result<()> {
        writer.write_application(23, |w| {
            w.write_context_integer(0, result.invocation_id as i64)?;
            Self::encode_context_bool(w, 1, result.success)?;

            if !result.result.is_empty() {
                w.write_context(2, |w2| {
                    for value in &result.result {
                        Self::encode_value(w2, value)?;
                    }
                    Ok(())
                })?;
            }

            Ok(())
        })
    }

    /// Encode a stream entry.
    fn encode_stream_entry(writer: &mut BerWriter, entry: &StreamEntry) -> Result<()> {
        writer.write_application(5, |w| {
            w.write_context_integer(0, entry.stream_identifier as i64)?;
            Self::encode_context_value(w, 1, &entry.value)?;
            Ok(())
        })
    }

    /// Encode a path as a RELATIVE-OID in a context tag.
    fn encode_context_path(writer: &mut BerWriter, tag: u32, path: &EmberPath) -> Result<()> {
        let components: Vec<u32> = path.iter().map(|&n| n as u32).collect();
        writer.write_context(tag, |w| {
            w.write_relative_oid(&components)
        })
    }

    /// Encode a string in a context tag (primitive, for backward compatibility).
    fn encode_context_string(writer: &mut BerWriter, tag: u32, value: &str) -> Result<()> {
        let tag_obj = Tag::new(TagClass::Context, TagType::Primitive, tag);
        writer.write_tlv(&tag_obj, value.as_bytes());
        Ok(())
    }

    /// Encode a UTF8String inside a constructed context tag.
    /// This is the correct format for Ember+ node contents: context[tag] -> UTF8String
    fn encode_context_utf8string(writer: &mut BerWriter, tag: u32, value: &str) -> Result<()> {
        writer.write_context(tag, |w| {
            w.write_utf8string(value);
            Ok(())
        })
    }

    /// Encode a boolean in a context tag.
    fn encode_context_bool(writer: &mut BerWriter, tag: u32, value: bool) -> Result<()> {
        let tag_obj = Tag::new(TagClass::Context, TagType::Primitive, tag);
        writer.write_tlv(&tag_obj, &[if value { 0xFF } else { 0x00 }]);
        Ok(())
    }

    /// Encode a value in a context tag.
    fn encode_context_value(writer: &mut BerWriter, tag: u32, value: &EmberValue) -> Result<()> {
        writer.write_context(tag, |w| {
            Self::encode_value(w, value)
        })
    }

    /// Encode a value.
    fn encode_value(writer: &mut BerWriter, value: &EmberValue) -> Result<()> {
        match value {
            EmberValue::Integer(v) => writer.write_integer(*v),
            EmberValue::Real(v) => writer.write_real(*v),
            EmberValue::String(v) => writer.write_utf8_string(v),
            EmberValue::Boolean(v) => writer.write_boolean(*v),
            EmberValue::Octets(v) => writer.write_octet_string(v),
            EmberValue::Null => writer.write_null(),
        }
    }
}
