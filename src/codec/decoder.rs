//! Glow decoder - converts BER bytes to Glow structures.

use crate::error::{GlowError, Result};
use crate::ber::{BerReader, Tag};
use crate::glow::{
    GlowRoot, GlowElement, GlowNode, GlowParameter, GlowFunction,
    GlowMatrix, GlowConnection, GlowCommand, GlowTemplate,
    InvocationResult, StreamEntry, EmberPath, EmberValue, 
    ParameterType, ParameterAccess, MatrixType, 
    MatrixAddressingMode, ConnectionOperation, ConnectionDisposition,
};

/// Decoder for Glow messages.
pub struct GlowDecoder;

impl GlowDecoder {
    /// Decode BER bytes to a GlowRoot.
    pub fn decode(data: &[u8]) -> Result<GlowRoot> {
        if data.is_empty() {
            return Ok(GlowRoot::new());
        }

        let mut reader = BerReader::new(data);
        Self::decode_root(&mut reader)
    }

    /// Decode the root element.
    fn decode_root(reader: &mut BerReader) -> Result<GlowRoot> {
        let mut root = GlowRoot::new();

        let tag = reader.peek_tag()?;
        
        // Application tag 0 = Root
        if tag.is_application() && tag.number == 0 {
            let mut inner = reader.read_application(0)?;
            
            while inner.has_more() {
                let child_tag = inner.peek_tag()?;
                
                if child_tag.is_application() {
                    match child_tag.number {
                        11 => {
                            // RootElementCollection
                            let mut coll = inner.read_application(11)?;
                            while coll.has_more() {
                                if let Some(element) = Self::decode_element(&mut coll)? {
                                    root.elements.push(element);
                                }
                            }
                        }
                        4 => {
                            // ElementCollection (legacy)
                            let mut coll = inner.read_application(4)?;
                            while coll.has_more() {
                                if let Some(element) = Self::decode_element(&mut coll)? {
                                    root.elements.push(element);
                                }
                            }
                        }
                        6 => {
                            // StreamCollection
                            let mut coll = inner.read_application(6)?;
                            while coll.has_more() {
                                if let Some(entry) = Self::decode_stream_entry(&mut coll)? {
                                    root.streams.push(entry);
                                }
                            }
                        }
                        23 => {
                            // InvocationResult
                            if let Some(result) = Self::decode_invocation_result(&mut inner)? {
                                root.invocation_results.push(result);
                            }
                        }
                        _ => {
                            // Try to decode as element
                            if let Some(element) = Self::decode_element(&mut inner)? {
                                root.elements.push(element);
                            }
                        }
                    }
                } else {
                    inner.skip()?;
                }
            }
        } else {
            // Try direct element decoding
            while reader.has_more() {
                if let Some(element) = Self::decode_element(reader)? {
                    root.elements.push(element);
                }
            }
        }

        Ok(root)
    }

    /// Decode an element.
    fn decode_element(reader: &mut BerReader) -> Result<Option<GlowElement>> {
        if !reader.has_more() {
            return Ok(None);
        }

        let tag = reader.peek_tag()?;

        // Handle context-tagged elements (NumberedTreeNode wrappers)
        if tag.is_context() {
            let mut inner = reader.read_context(tag.number)?;
            // Inside the context wrapper, look for the actual element
            if inner.has_more() {
                return Self::decode_element(&mut inner);
            }
            return Ok(None);
        }

        if !tag.is_application() {
            reader.skip()?;
            return Ok(None);
        }

        match tag.number {
            1 => Ok(Some(Self::decode_parameter(reader)?)),
            2 => Ok(Some(Self::decode_command(reader)?)),
            3 => Ok(Some(Self::decode_node(reader)?)),
            9 => Ok(Some(Self::decode_qualified_parameter(reader)?)),
            10 => Ok(Some(Self::decode_qualified_node(reader)?)),
            13 => Ok(Some(Self::decode_matrix(reader)?)),
            17 => Ok(Some(Self::decode_qualified_matrix(reader)?)),
            19 => Ok(Some(Self::decode_function(reader)?)),
            20 => Ok(Some(Self::decode_qualified_function(reader)?)),
            24 => Ok(Some(Self::decode_template(reader)?)),
            _ => {
                reader.skip()?;
                Ok(None)
            }
        }
    }

    /// Decode a node.
    fn decode_node(reader: &mut BerReader) -> Result<GlowElement> {
        let mut inner = reader.read_application(3)?;
        let mut node = GlowNode::new(0);

        while inner.has_more() {
            let tag = inner.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => {
                        node.number = inner.read_context_integer(0)? as i32;
                    }
                    1 => {
                        let mut contents = inner.read_context(1)?;
                        Self::decode_node_contents(&mut contents, &mut node)?;
                    }
                    2 => {
                        let mut children_ctx = inner.read_context(2)?;
                        if children_ctx.has_more() {
                            let child_tag = children_ctx.peek_tag()?;
                            if child_tag.is_application() && child_tag.number == 4 {
                                let mut coll = children_ctx.read_application(4)?;
                                while coll.has_more() {
                                    if let Some(child) = Self::decode_element(&mut coll)? {
                                        node.children.push(child);
                                    }
                                }
                            }
                        }
                    }
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        Ok(GlowElement::Node(node))
    }

    /// Decode node contents.
    fn decode_node_contents(reader: &mut BerReader, node: &mut GlowNode) -> Result<()> {
        // Server may wrap contents in a SET (0x31) or SEQUENCE (0x30)
        let tag = reader.peek_tag()?;
        if tag.is_universal() && (tag.number == 17 || tag.number == 16) {
            // SET or SEQUENCE - read the TLV and create inner reader
            let (_, value) = reader.read_tlv()?;
            let mut inner = BerReader::new(value);
            Self::decode_node_contents_inner(&mut inner, node)?;
        } else {
            // Direct contents
            Self::decode_node_contents_inner(reader, node)?;
        }
        Ok(())
    }
    
    /// Decode node contents inner.
    fn decode_node_contents_inner(reader: &mut BerReader, node: &mut GlowNode) -> Result<()> {
        while reader.has_more() {
            let tag = reader.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => node.identifier = Some(Self::read_context_string(reader, 0)?),
                    1 => node.description = Some(Self::read_context_string(reader, 1)?),
                    2 => node.is_root = Some(Self::read_context_bool(reader, 2)?),
                    3 => node.is_online = Some(Self::read_context_bool(reader, 3)?),
                    4 => node.schema_identifiers = Some(Self::read_context_string(reader, 4)?),
                    _ => { reader.skip()?; }
                }
            } else {
                reader.skip()?;
            }
        }
        Ok(())
    }

    /// Decode a parameter.
    fn decode_parameter(reader: &mut BerReader) -> Result<GlowElement> {
        let mut inner = reader.read_application(1)?;
        let mut param = GlowParameter::new(0);

        while inner.has_more() {
            let tag = inner.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => {
                        param.number = inner.read_context_integer(0)? as i32;
                    }
                    1 => {
                        let mut contents = inner.read_context(1)?;
                        Self::decode_parameter_contents(&mut contents, &mut param)?;
                    }
                    2 => {
                        let mut children_ctx = inner.read_context(2)?;
                        if children_ctx.has_more() {
                            let child_tag = children_ctx.peek_tag()?;
                            if child_tag.is_application() && child_tag.number == 4 {
                                let mut coll = children_ctx.read_application(4)?;
                                while coll.has_more() {
                                    if let Some(child) = Self::decode_element(&mut coll)? {
                                        param.children.push(child);
                                    }
                                }
                            }
                        }
                    }
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        Ok(GlowElement::Parameter(param))
    }

    /// Decode parameter contents.
    fn decode_parameter_contents(reader: &mut BerReader, param: &mut GlowParameter) -> Result<()> {
        // Server may wrap contents in a SET (0x31 = tag 17) or SEQUENCE (0x30 = tag 16)
        let tag = reader.peek_tag()?;
        if tag.is_universal() && (tag.number == 17 || tag.number == 16) {
            // SET or SEQUENCE wrapper - read the TLV and create inner reader
            let (_, value) = reader.read_tlv()?;
            let mut inner = BerReader::new(value);
            return Self::decode_parameter_contents_inner(&mut inner, param);
        }
        
        Self::decode_parameter_contents_inner(reader, param)
    }
    
    /// Decode parameter contents inner (after unwrapping SET/SEQUENCE if present).
    fn decode_parameter_contents_inner(reader: &mut BerReader, param: &mut GlowParameter) -> Result<()> {
        while reader.has_more() {
            let tag = reader.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => param.identifier = Some(Self::read_context_string(reader, 0)?),
                    1 => param.description = Some(Self::read_context_string(reader, 1)?),
                    2 => param.value = Some(Self::read_context_value(reader, 2)?),
                    3 => param.minimum = Some(Self::read_context_value(reader, 3)?),
                    4 => param.maximum = Some(Self::read_context_value(reader, 4)?),
                    5 => param.access = Some(ParameterAccess::try_from(reader.read_context_integer(5)?)?),
                    6 => param.format = Some(Self::read_context_string(reader, 6)?),
                    7 => param.enumeration = Some(Self::read_context_string(reader, 7)?),
                    8 => param.factor = Some(reader.read_context_integer(8)? as i32),
                    9 => param.is_online = Some(Self::read_context_bool(reader, 9)?),
                    10 => param.formula = Some(Self::read_context_string(reader, 10)?),
                    11 => param.step = Some(reader.read_context_integer(11)? as i32),
                    12 => param.default = Some(Self::read_context_value(reader, 12)?),
                    13 => param.parameter_type = Some(ParameterType::try_from(reader.read_context_integer(13)?)?),
                    14 => param.stream_identifier = Some(reader.read_context_integer(14)? as i32),
                    _ => { reader.skip()?; }
                }
            } else {
                reader.skip()?;
            }
        }
        
        Ok(())
    }

    /// Decode function contents, handling optional SET (0x31) wrapper from Lawo consoles.
    fn decode_function_contents(reader: &mut BerReader, func: &mut GlowFunction) -> Result<()> {
        // Lawo consoles wrap Function contents in a SET (0x31) — unwrap if present
        if reader.has_more() {
            let tag = reader.peek_tag()?;
            if tag.is_universal() && (tag.number == 17 || tag.number == 16) {
                let (_, data) = reader.read_tlv()?;
                let mut inner = BerReader::new(data);
                return Self::decode_function_contents_inner(&mut inner, func);
            }
        }
        Self::decode_function_contents_inner(reader, func)
    }

    fn decode_function_contents_inner(reader: &mut BerReader, func: &mut GlowFunction) -> Result<()> {
        while reader.has_more() {
            let ctag = reader.peek_tag()?;
            if ctag.is_context() {
                match ctag.number {
                    0 => func.identifier = Some(Self::read_context_string(reader, 0)?),
                    1 => func.description = Some(Self::read_context_string(reader, 1)?),
                    _ => { reader.skip()?; }
                }
            } else {
                reader.skip()?;
            }
        }
        Ok(())
    }

    /// Decode a function.
    fn decode_function(reader: &mut BerReader) -> Result<GlowElement> {
        let mut inner = reader.read_application(19)?;
        let mut func = GlowFunction::new(0);

        while inner.has_more() {
            let tag = inner.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => func.number = inner.read_context_integer(0)? as i32,
                    1 => {
                        let mut contents = inner.read_context(1)?;
                        Self::decode_function_contents(&mut contents, &mut func)?;
                    }
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        Ok(GlowElement::Function(func))
    }

    /// Decode a matrix.
    fn decode_matrix(reader: &mut BerReader) -> Result<GlowElement> {
        let mut inner = reader.read_application(13)?;
        let mut matrix = GlowMatrix::new(0);

        while inner.has_more() {
            let tag = inner.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => matrix.number = inner.read_context_integer(0)? as i32,
                    1 => {
                        let mut contents = inner.read_context(1)?;
                        Self::decode_matrix_contents(&mut contents, &mut matrix)?;
                    }
                    5 => {
                        let mut conns_ctx = inner.read_context(5)?;
                        Self::decode_matrix_connections(&mut conns_ctx, &mut matrix)?
                    }
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        Ok(GlowElement::Matrix(matrix))
    }

    /// Decode connections from context[5] of a matrix.
    /// Lawo format: context[5] → SEQUENCE → context[0] → Connection(app16)
    fn decode_matrix_connections(reader: &mut BerReader, matrix: &mut GlowMatrix) -> Result<()> {
        // Unwrap outer SEQUENCE (0x30) or SET (0x31) if present
        let mut work_reader;
        let reader: &mut BerReader = if reader.has_more() {
            let tag = reader.peek_tag()?;
            if tag.is_universal() && (tag.number == 16 || tag.number == 17) {
                let (_, value) = reader.read_tlv()?;
                work_reader = BerReader::new(value);
                &mut work_reader
            } else {
                reader
            }
        } else {
            return Ok(());
        };

        while reader.has_more() {
            let tag = reader.peek_tag()?;
            if tag.is_context() {
                // context[0] wraps a Connection app(16)
                let mut ctx = reader.read_context(tag.number)?;
                if ctx.has_more() {
                    if let Some(conn) = Self::decode_connection(&mut ctx)? {
                        matrix.connections.push(conn);
                    }
                }
            } else if tag.is_application() && tag.number == 16 {
                // Direct Connection app(16)
                if let Some(conn) = Self::decode_connection(reader)? {
                    matrix.connections.push(conn);
                }
            } else {
                reader.skip()?;
            }
        }
        Ok(())
    }

    /// Decode matrix contents, handling optional SET (0x31) or SEQUENCE (0x30) wrapper.
    fn decode_matrix_contents(reader: &mut BerReader, matrix: &mut GlowMatrix) -> Result<()> {
        if reader.has_more() {
            let tag = reader.peek_tag()?;
            // Lawo consoles wrap contents in SET (0x31) or SEQUENCE (0x30)
            if tag.is_universal() && (tag.number == 17 || tag.number == 16) {
                let (_, value) = reader.read_tlv()?;
                let mut inner = BerReader::new(value);
                return Self::decode_matrix_contents_inner(&mut inner, matrix);
            }
        }
        Self::decode_matrix_contents_inner(reader, matrix)
    }

    /// Inner matrix contents decoder (context tags directly).
    fn decode_matrix_contents_inner(reader: &mut BerReader, matrix: &mut GlowMatrix) -> Result<()> {
        while reader.has_more() {
            let tag = reader.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => matrix.identifier = Some(Self::read_context_string(reader, 0)?),
                    1 => matrix.description = Some(Self::read_context_string(reader, 1)?),
                    2 => matrix.matrix_type = Some(MatrixType::try_from(reader.read_context_integer(2)?)?),
                    3 => matrix.addressing_mode = Some(MatrixAddressingMode::try_from(reader.read_context_integer(3)?)?),
                    4 => matrix.target_count = Some(reader.read_context_integer(4)? as i32),
                    5 => matrix.source_count = Some(reader.read_context_integer(5)? as i32),
                    6 => matrix.max_connections_per_target = Some(reader.read_context_integer(6)? as i32),
                    7 => matrix.max_total_connections = Some(reader.read_context_integer(7)? as i32),
                    _ => { reader.skip()?; }
                }
            } else {
                reader.skip()?;
            }
        }
        Ok(())
    }

    /// Decode a connection.
    fn decode_connection(reader: &mut BerReader) -> Result<Option<GlowConnection>> {
        let tag = reader.peek_tag()?;
        if !tag.is_application() || tag.number != 16 {
            reader.skip()?;
            return Ok(None);
        }

        let mut inner = reader.read_application(16)?;
        let mut conn = GlowConnection::default();

        while inner.has_more() {
            let ctag = inner.peek_tag()?;
            if ctag.is_context() {
                match ctag.number {
                    0 => conn.target = inner.read_context_integer(0)? as i32,
                    1 => {
                        let mut sources = inner.read_context(1)?;
                        if sources.has_more() {
                            let src_tag = sources.peek_tag()?;
                            if src_tag.is_universal() && src_tag.number == 13 {
                                // RELATIVE-OID encoding of source list
                                let oid = sources.read_relative_oid()?;
                                for v in oid { conn.sources.push(v as i32); }
                            } else {
                                // Fallback: sequence of integers
                                while sources.has_more() {
                                    conn.sources.push(sources.read_integer()? as i32);
                                }
                            }
                        }
                    }
                    2 => conn.operation = Some(ConnectionOperation::try_from(inner.read_context_integer(2)?)?),
                    3 => conn.disposition = Some(ConnectionDisposition::try_from(inner.read_context_integer(3)?)?),
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        Ok(Some(conn))
    }

    /// Decode a command.
    fn decode_command(reader: &mut BerReader) -> Result<GlowElement> {
        let mut inner = reader.read_application(2)?;
        let mut cmd_number = 0i32;
        let mut invocation_id = 0i32;
        let mut arguments = Vec::new();

        while inner.has_more() {
            let tag = inner.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => cmd_number = inner.read_context_integer(0)? as i32,
                    2 => {
                        let mut inv = inner.read_context(2)?;
                        while inv.has_more() {
                            let itag = inv.peek_tag()?;
                            if itag.is_context() {
                                match itag.number {
                                    0 => invocation_id = inv.read_context_integer(0)? as i32,
                                    1 => {
                                        let mut args = inv.read_context(1)?;
                                        while args.has_more() {
                                            arguments.push(Self::decode_value(&mut args)?);
                                        }
                                    }
                                    _ => { inv.skip()?; }
                                }
                            } else {
                                inv.skip()?;
                            }
                        }
                    }
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        let command = match cmd_number {
            1 => GlowCommand::Subscribe,
            2 => GlowCommand::Unsubscribe,
            32 => GlowCommand::GetDirectory,
            33 => GlowCommand::Invoke { invocation_id, arguments },
            _ => GlowCommand::GetDirectory, // Default
        };

        Ok(GlowElement::Command(command))
    }

    /// Decode a qualified node.
    fn decode_qualified_node(reader: &mut BerReader) -> Result<GlowElement> {
        let mut inner = reader.read_application(10)?;
        let mut path = Vec::new();
        let mut node = GlowNode::new(0);

        while inner.has_more() {
            let tag = inner.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => path = Self::read_context_path(&mut inner, 0)?,
                    1 => {
                        let mut contents = inner.read_context(1)?;
                        Self::decode_node_contents(&mut contents, &mut node)?;
                    }
                    2 => {
                        let mut children_ctx = inner.read_context(2)?;
                        if children_ctx.has_more() {
                            let child_tag = children_ctx.peek_tag()?;
                            if child_tag.is_application() && child_tag.number == 4 {
                                let mut coll = children_ctx.read_application(4)?;
                                while coll.has_more() {
                                    if let Some(child) = Self::decode_element(&mut coll)? {
                                        node.children.push(child);
                                    }
                                }
                            }
                        }
                    }
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        if let Some(&last) = path.last() {
            node.number = last;
        }

        Ok(GlowElement::QualifiedNode(path, node))
    }

    /// Decode a qualified parameter.
    fn decode_qualified_parameter(reader: &mut BerReader) -> Result<GlowElement> {
        let mut inner = reader.read_application(9)?;
        let mut path = Vec::new();
        let mut param = GlowParameter::new(0);

        while inner.has_more() {
            let tag = inner.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => path = Self::read_context_path(&mut inner, 0)?,
                    1 => {
                        let mut contents = inner.read_context(1)?;
                        Self::decode_parameter_contents(&mut contents, &mut param)?;
                    }
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        if let Some(&last) = path.last() {
            param.number = last;
        }

        Ok(GlowElement::QualifiedParameter(path, param))
    }

    /// Decode a qualified function.
    fn decode_qualified_function(reader: &mut BerReader) -> Result<GlowElement> {
        let mut inner = reader.read_application(20)?;
        let mut path = Vec::new();
        let mut func = GlowFunction::new(0);

        while inner.has_more() {
            let tag = inner.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => path = Self::read_context_path(&mut inner, 0)?,
                    1 => {
                        let mut contents = inner.read_context(1)?;
                        Self::decode_function_contents(&mut contents, &mut func)?;
                    }
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        if let Some(&last) = path.last() {
            func.number = last;
        }

        Ok(GlowElement::QualifiedFunction(path, func))
    }

    /// Decode a qualified matrix.
    fn decode_qualified_matrix(reader: &mut BerReader) -> Result<GlowElement> {
        let mut inner = reader.read_application(17)?;
        let mut path = Vec::new();
        let mut matrix = GlowMatrix::new(0);

        while inner.has_more() {
            let tag = inner.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => path = Self::read_context_path(&mut inner, 0)?,
                    1 => {
                        let mut contents = inner.read_context(1)?;
                        Self::decode_matrix_contents(&mut contents, &mut matrix)?;
                    }
                    5 => {
                        let mut conns_ctx = inner.read_context(5)?;
                        Self::decode_matrix_connections(&mut conns_ctx, &mut matrix)?
                    }
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        if let Some(&last) = path.last() {
            matrix.number = last;
        }

        Ok(GlowElement::QualifiedMatrix(path, matrix))
    }

    /// Decode a template.
    fn decode_template(reader: &mut BerReader) -> Result<GlowElement> {
        let mut inner = reader.read_application(24)?;
        let mut template = GlowTemplate::default();

        while inner.has_more() {
            let tag = inner.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => template.number = inner.read_context_integer(0)? as i32,
                    2 => template.description = Some(Self::read_context_string(&mut inner, 2)?),
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        Ok(GlowElement::Template(template))
    }

    /// Decode an invocation result.
    fn decode_invocation_result(reader: &mut BerReader) -> Result<Option<InvocationResult>> {
        let mut inner = reader.read_application(23)?;
        let mut invocation_id = 0;
        let mut success = false;
        let mut result = Vec::new();

        while inner.has_more() {
            let tag = inner.peek_tag()?;
            
            if tag.is_context() {
                match tag.number {
                    0 => invocation_id = inner.read_context_integer(0)? as i32,
                    1 => success = Self::read_context_bool(&mut inner, 1)?,
                    2 => {
                        // context[2] contains a SEQUENCE of CONTEXT(0)-wrapped values
                        let mut res = inner.read_context(2)?;
                        if res.has_more() {
                            let seq_tag = res.peek_tag()?;
                            // Unwrap the SEQUENCE if present
                            if seq_tag.is_universal() && seq_tag.number == 16 {
                                let mut seq = res.read_sequence()?;
                                while seq.has_more() {
                                    let item_tag = seq.peek_tag()?;
                                    if item_tag.is_context() && item_tag.number == 0 {
                                        let mut item = seq.read_context(0)?;
                                        result.push(Self::decode_value(&mut item)?);
                                    } else {
                                        seq.skip()?;
                                    }
                                }
                            } else {
                                // Fallback: read values directly
                                while res.has_more() {
                                    result.push(Self::decode_value(&mut res)?);
                                }
                            }
                        }
                    }
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        Ok(Some(InvocationResult { invocation_id, success, result }))
    }

    /// Decode a stream entry.
    fn decode_stream_entry(reader: &mut BerReader) -> Result<Option<StreamEntry>> {
        let tag = reader.peek_tag()?;
        
        // StreamEntry can be wrapped in CONTEXT 0 or be direct APPLICATION 5
        let mut inner = if tag.is_context() && tag.number == 0 {
            // Unwrap from context[0]
            let mut ctx = reader.read_context(0)?;
            let inner_tag = ctx.peek_tag()?;
            if !inner_tag.is_application() || inner_tag.number != 5 {
                return Ok(None);
            }
            ctx.read_application(5)?
        } else if tag.is_application() && tag.number == 5 {
            reader.read_application(5)?
        } else {
            reader.skip()?;
            return Ok(None);
        };
        let mut stream_identifier = 0;
        let mut value = EmberValue::Null;

        while inner.has_more() {
            let ctag = inner.peek_tag()?;
            if ctag.is_context() {
                match ctag.number {
                    0 => stream_identifier = inner.read_context_integer(0)? as i32,
                    1 => value = Self::read_context_value(&mut inner, 1)?,
                    _ => { inner.skip()?; }
                }
            } else {
                inner.skip()?;
            }
        }

        Ok(Some(StreamEntry { stream_identifier, value }))
    }

    /// Read a context-tagged string.
    fn read_context_string(reader: &mut BerReader, _tag: u32) -> Result<String> {
        let (_, data) = reader.read_tlv()?;
        // The context tag contains a UTF8String (0x0C) or other string type
        // Parse the inner TLV to get the actual string
        if data.len() >= 2 {
            let mut inner = BerReader::new(data);
            let (inner_tag, inner_data) = inner.read_tlv()?;
            // UTF8String (0x0C) or other string types
            if inner_tag.is_universal() {
                return Ok(String::from_utf8_lossy(inner_data).to_string());
            }
        }
        // Fallback: treat as raw string
        Ok(String::from_utf8_lossy(data).to_string())
    }

    /// Read a context-tagged boolean.
    fn read_context_bool(reader: &mut BerReader, _tag: u32) -> Result<bool> {
        let (_, data) = reader.read_tlv()?;
        Ok(data.first().map(|&b| b != 0).unwrap_or(false))
    }

    /// Read a context-tagged path.
    fn read_context_path(reader: &mut BerReader, _tag: u32) -> Result<EmberPath> {
        let mut ctx = reader.read_context(0)?;
        let oid = ctx.read_relative_oid()?;
        Ok(oid.into_iter().map(|n| n as i32).collect())
    }

    /// Read a context-tagged value.
    fn read_context_value(reader: &mut BerReader, _tag: u32) -> Result<EmberValue> {
        let mut ctx = reader.read_context(_tag)?;
        Self::decode_value(&mut ctx)
    }

    /// Decode a value.
    fn decode_value(reader: &mut BerReader) -> Result<EmberValue> {
        let (tag, data) = reader.read_tlv()?;

        if tag.is_universal() {
            match tag.number {
                1 => Ok(EmberValue::Boolean(data.first().map(|&b| b != 0).unwrap_or(false))),
                2 => {
                    let value = Self::decode_integer(data)?;
                    Ok(EmberValue::Integer(value))
                }
                4 => Ok(EmberValue::Octets(data.to_vec())),
                5 => Ok(EmberValue::Null),
                9 => {
                    let value = Self::decode_real(data)?;
                    Ok(EmberValue::Real(value))
                }
                12 => Ok(EmberValue::String(String::from_utf8_lossy(data).to_string())),
                _ => Ok(EmberValue::Null),
            }
        } else {
            Ok(EmberValue::Octets(data.to_vec()))
        }
    }

    /// Decode an integer from bytes.
    fn decode_integer(data: &[u8]) -> Result<i64> {
        if data.is_empty() {
            return Ok(0);
        }

        let negative = (data[0] & 0x80) != 0;
        let mut result: i64 = if negative { -1 } else { 0 };

        for &byte in data {
            result = (result << 8) | (byte as i64);
        }

        Ok(result)
    }

    /// Decode a real from bytes.
    fn decode_real(data: &[u8]) -> Result<f64> {
        if data.is_empty() {
            return Ok(0.0);
        }

        let first = data[0];
        
        if first == 0x40 { return Ok(f64::INFINITY); }
        if first == 0x41 { return Ok(f64::NEG_INFINITY); }
        if first == 0x42 { return Ok(f64::NAN); }
        if first == 0x43 { return Ok(-0.0); }

        // Binary encoding
        if (first & 0x80) != 0 {
            let sign = if (first & 0x40) != 0 { -1.0 } else { 1.0 };
            let exp_len = match first & 0x03 {
                0 => 1, 1 => 2, 2 => 3,
                _ => data.get(1).copied().unwrap_or(0) as usize,
            };
            let exp_start = if (first & 0x03) == 3 { 2 } else { 1 };
            
            if data.len() < exp_start + exp_len {
                return Ok(0.0);
            }

            let exp_bytes = &data[exp_start..exp_start + exp_len];
            let mut exp: i64 = if !exp_bytes.is_empty() && (exp_bytes[0] & 0x80) != 0 { -1 } else { 0 };
            for &b in exp_bytes { exp = (exp << 8) | (b as i64); }

            let mant_bytes = &data[exp_start + exp_len..];
            let mut mant: u64 = 0;
            for &b in mant_bytes { mant = (mant << 8) | (b as u64); }

            return Ok(sign * (mant as f64) * 2.0_f64.powi(exp as i32));
        }

        Ok(0.0)
    }
}
