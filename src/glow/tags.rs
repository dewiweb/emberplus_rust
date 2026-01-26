//! Glow DTD tag definitions.
//!
//! These tags define the application-specific tags used in the Ember+ protocol.

use crate::ber::{Tag, TagClass, TagType};

/// Application tags for Glow elements.
pub mod app {
    use super::*;

    /// Root element containing the entire tree or a subset.
    pub const ROOT: Tag = Tag::new(TagClass::Application, TagType::Constructed, 0);
    
    /// Element collection (children of a node).
    pub const ELEMENT_COLLECTION: Tag = Tag::new(TagClass::Application, TagType::Constructed, 4);
    
    /// Root element collection.
    pub const ROOT_ELEMENT_COLLECTION: Tag = Tag::new(TagClass::Application, TagType::Constructed, 11);
    
    /// Stream collection.
    pub const STREAM_COLLECTION: Tag = Tag::new(TagClass::Application, TagType::Constructed, 6);
    
    /// Stream entry.
    pub const STREAM_ENTRY: Tag = Tag::new(TagClass::Application, TagType::Constructed, 5);
    
    /// Invocation result.
    pub const INVOCATION_RESULT: Tag = Tag::new(TagClass::Application, TagType::Constructed, 23);
}

/// Context tags for Parameter.
pub mod parameter {
    use super::*;

    /// Parameter element.
    pub const ELEMENT: Tag = Tag::new(TagClass::Application, TagType::Constructed, 1);
    
    /// Qualified parameter (with full path).
    pub const QUALIFIED: Tag = Tag::new(TagClass::Application, TagType::Constructed, 9);
    
    /// Parameter number.
    pub const NUMBER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Parameter contents.
    pub const CONTENTS: Tag = Tag::new(TagClass::Context, TagType::Constructed, 1);
    
    /// Parameter children.
    pub const CHILDREN: Tag = Tag::new(TagClass::Context, TagType::Constructed, 2);
    
    // Contents fields
    /// Identifier string.
    pub const IDENTIFIER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    /// Description string.
    pub const DESCRIPTION: Tag = Tag::new(TagClass::Context, TagType::Primitive, 1);
    /// Current value.
    pub const VALUE: Tag = Tag::new(TagClass::Context, TagType::Primitive, 2);
    /// Minimum value.
    pub const MINIMUM: Tag = Tag::new(TagClass::Context, TagType::Primitive, 3);
    /// Maximum value.
    pub const MAXIMUM: Tag = Tag::new(TagClass::Context, TagType::Primitive, 4);
    /// Access mode.
    pub const ACCESS: Tag = Tag::new(TagClass::Context, TagType::Primitive, 5);
    /// Format string.
    pub const FORMAT: Tag = Tag::new(TagClass::Context, TagType::Primitive, 6);
    /// Enumeration map.
    pub const ENUMERATION: Tag = Tag::new(TagClass::Context, TagType::Primitive, 7);
    /// Factor for value conversion.
    pub const FACTOR: Tag = Tag::new(TagClass::Context, TagType::Primitive, 8);
    /// Is online.
    pub const IS_ONLINE: Tag = Tag::new(TagClass::Context, TagType::Primitive, 9);
    /// Formula for value transformation.
    pub const FORMULA: Tag = Tag::new(TagClass::Context, TagType::Primitive, 10);
    /// Step size.
    pub const STEP: Tag = Tag::new(TagClass::Context, TagType::Primitive, 11);
    /// Default value.
    pub const DEFAULT: Tag = Tag::new(TagClass::Context, TagType::Primitive, 12);
    /// Parameter type.
    pub const TYPE: Tag = Tag::new(TagClass::Context, TagType::Primitive, 13);
    /// Stream identifier.
    pub const STREAM_IDENTIFIER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 14);
    /// Enumeration map (string-based).
    pub const ENUM_MAP: Tag = Tag::new(TagClass::Context, TagType::Constructed, 15);
    /// Stream descriptor.
    pub const STREAM_DESCRIPTOR: Tag = Tag::new(TagClass::Context, TagType::Constructed, 16);
    /// Schema identifiers.
    pub const SCHEMA_IDENTIFIERS: Tag = Tag::new(TagClass::Context, TagType::Primitive, 17);
    /// Template reference.
    pub const TEMPLATE_REFERENCE: Tag = Tag::new(TagClass::Context, TagType::Primitive, 18);
}

/// Context tags for Node.
pub mod node {
    use super::*;

    /// Node element.
    pub const ELEMENT: Tag = Tag::new(TagClass::Application, TagType::Constructed, 3);
    
    /// Qualified node (with full path).
    pub const QUALIFIED: Tag = Tag::new(TagClass::Application, TagType::Constructed, 10);
    
    /// Node number.
    pub const NUMBER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Node contents.
    pub const CONTENTS: Tag = Tag::new(TagClass::Context, TagType::Constructed, 1);
    
    /// Node children.
    pub const CHILDREN: Tag = Tag::new(TagClass::Context, TagType::Constructed, 2);
    
    // Contents fields
    /// Identifier string.
    pub const IDENTIFIER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    /// Description string.
    pub const DESCRIPTION: Tag = Tag::new(TagClass::Context, TagType::Primitive, 1);
    /// Is root node.
    pub const IS_ROOT: Tag = Tag::new(TagClass::Context, TagType::Primitive, 2);
    /// Is online.
    pub const IS_ONLINE: Tag = Tag::new(TagClass::Context, TagType::Primitive, 3);
    /// Schema identifiers.
    pub const SCHEMA_IDENTIFIERS: Tag = Tag::new(TagClass::Context, TagType::Primitive, 4);
    /// Template reference.
    pub const TEMPLATE_REFERENCE: Tag = Tag::new(TagClass::Context, TagType::Primitive, 5);
}

/// Context tags for Command.
pub mod command {
    use super::*;

    /// Command element.
    pub const ELEMENT: Tag = Tag::new(TagClass::Application, TagType::Constructed, 2);
    
    /// Command number (type).
    pub const NUMBER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Command options/dirfield.
    pub const DIRFIELD_MASK: Tag = Tag::new(TagClass::Context, TagType::Primitive, 1);
    
    /// Invocation (for function calls).
    pub const INVOCATION: Tag = Tag::new(TagClass::Context, TagType::Constructed, 2);
}

/// Context tags for Function.
pub mod function {
    use super::*;

    /// Function element.
    pub const ELEMENT: Tag = Tag::new(TagClass::Application, TagType::Constructed, 19);
    
    /// Qualified function (with full path).
    pub const QUALIFIED: Tag = Tag::new(TagClass::Application, TagType::Constructed, 20);
    
    /// Function number.
    pub const NUMBER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Function contents.
    pub const CONTENTS: Tag = Tag::new(TagClass::Context, TagType::Constructed, 1);
    
    /// Function children.
    pub const CHILDREN: Tag = Tag::new(TagClass::Context, TagType::Constructed, 2);
    
    // Contents fields
    /// Identifier string.
    pub const IDENTIFIER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    /// Description string.
    pub const DESCRIPTION: Tag = Tag::new(TagClass::Context, TagType::Primitive, 1);
    /// Arguments tuple description.
    pub const ARGUMENTS: Tag = Tag::new(TagClass::Context, TagType::Constructed, 2);
    /// Result tuple description.
    pub const RESULT: Tag = Tag::new(TagClass::Context, TagType::Constructed, 3);
    /// Template reference.
    pub const TEMPLATE_REFERENCE: Tag = Tag::new(TagClass::Context, TagType::Primitive, 4);
}

/// Context tags for Matrix.
pub mod matrix {
    use super::*;

    /// Matrix element.
    pub const ELEMENT: Tag = Tag::new(TagClass::Application, TagType::Constructed, 13);
    
    /// Qualified matrix (with full path).
    pub const QUALIFIED: Tag = Tag::new(TagClass::Application, TagType::Constructed, 17);
    
    /// Matrix number.
    pub const NUMBER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Matrix contents.
    pub const CONTENTS: Tag = Tag::new(TagClass::Context, TagType::Constructed, 1);
    
    /// Matrix children.
    pub const CHILDREN: Tag = Tag::new(TagClass::Context, TagType::Constructed, 2);
    
    /// Target connections.
    pub const TARGETS: Tag = Tag::new(TagClass::Context, TagType::Constructed, 3);
    
    /// Source connections.
    pub const SOURCES: Tag = Tag::new(TagClass::Context, TagType::Constructed, 4);
    
    /// Matrix connections.
    pub const CONNECTIONS: Tag = Tag::new(TagClass::Context, TagType::Constructed, 5);
    
    // Contents fields
    /// Identifier string.
    pub const IDENTIFIER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    /// Description string.
    pub const DESCRIPTION: Tag = Tag::new(TagClass::Context, TagType::Primitive, 1);
    /// Matrix type.
    pub const TYPE: Tag = Tag::new(TagClass::Context, TagType::Primitive, 2);
    /// Addressing mode.
    pub const ADDRESSING_MODE: Tag = Tag::new(TagClass::Context, TagType::Primitive, 3);
    /// Target count.
    pub const TARGET_COUNT: Tag = Tag::new(TagClass::Context, TagType::Primitive, 4);
    /// Source count.
    pub const SOURCE_COUNT: Tag = Tag::new(TagClass::Context, TagType::Primitive, 5);
    /// Maximum connections per target.
    pub const MAX_CONNECTIONS_PER_TARGET: Tag = Tag::new(TagClass::Context, TagType::Primitive, 6);
    /// Maximum connections total.
    pub const MAX_TOTAL_CONNECTIONS: Tag = Tag::new(TagClass::Context, TagType::Primitive, 7);
    /// Connection parameters location.
    pub const CONNECTION_PARAMETERS_LOCATION: Tag = Tag::new(TagClass::Context, TagType::Primitive, 8);
    /// Gain parameter number.
    pub const GAIN_PARAMETER_NUMBER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 9);
    /// Labels.
    pub const LABELS: Tag = Tag::new(TagClass::Context, TagType::Constructed, 10);
    /// Schema identifiers.
    pub const SCHEMA_IDENTIFIERS: Tag = Tag::new(TagClass::Context, TagType::Primitive, 11);
    /// Template reference.
    pub const TEMPLATE_REFERENCE: Tag = Tag::new(TagClass::Context, TagType::Primitive, 12);
}

/// Context tags for Signal (matrix target/source).
pub mod signal {
    use super::*;

    /// Signal element.
    pub const ELEMENT: Tag = Tag::new(TagClass::Application, TagType::Constructed, 14);
    
    /// Signal number.
    pub const NUMBER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
}

/// Context tags for Connection.
pub mod connection {
    use super::*;

    /// Connection element.
    pub const ELEMENT: Tag = Tag::new(TagClass::Application, TagType::Constructed, 16);
    
    /// Target number.
    pub const TARGET: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Source numbers.
    pub const SOURCES: Tag = Tag::new(TagClass::Context, TagType::Constructed, 1);
    
    /// Operation.
    pub const OPERATION: Tag = Tag::new(TagClass::Context, TagType::Primitive, 2);
    
    /// Disposition.
    pub const DISPOSITION: Tag = Tag::new(TagClass::Context, TagType::Primitive, 3);
}

/// Context tags for Label.
pub mod label {
    use super::*;

    /// Label element.
    pub const ELEMENT: Tag = Tag::new(TagClass::Application, TagType::Constructed, 18);
    
    /// Base path.
    pub const BASE_PATH: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Description.
    pub const DESCRIPTION: Tag = Tag::new(TagClass::Context, TagType::Primitive, 1);
}

/// Context tags for Invocation.
pub mod invocation {
    use super::*;

    /// Invocation ID.
    pub const ID: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Arguments.
    pub const ARGUMENTS: Tag = Tag::new(TagClass::Context, TagType::Constructed, 1);
}

/// Context tags for InvocationResult.
pub mod invocation_result {
    use super::*;

    /// Invocation ID.
    pub const ID: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Success flag.
    pub const SUCCESS: Tag = Tag::new(TagClass::Context, TagType::Primitive, 1);
    
    /// Result values.
    pub const RESULT: Tag = Tag::new(TagClass::Context, TagType::Constructed, 2);
}

/// Context tags for StreamEntry.
pub mod stream_entry {
    use super::*;

    /// Stream identifier.
    pub const IDENTIFIER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Stream value.
    pub const VALUE: Tag = Tag::new(TagClass::Context, TagType::Primitive, 1);
}

/// Context tags for Tuple.
pub mod tuple {
    use super::*;

    /// Tuple item description.
    pub const ITEM: Tag = Tag::new(TagClass::Application, TagType::Constructed, 21);
    
    /// Item type.
    pub const TYPE: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Item name.
    pub const NAME: Tag = Tag::new(TagClass::Context, TagType::Primitive, 1);
}

/// Context tags for Template.
pub mod template {
    use super::*;

    /// Template element.
    pub const ELEMENT: Tag = Tag::new(TagClass::Application, TagType::Constructed, 24);
    
    /// Template number.
    pub const NUMBER: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Template element definition.
    pub const ELEMENT_DEF: Tag = Tag::new(TagClass::Context, TagType::Constructed, 1);
    
    /// Template description.
    pub const DESCRIPTION: Tag = Tag::new(TagClass::Context, TagType::Primitive, 2);
}

/// Context tags for QualifiedParameter/Node/etc.
pub mod qualified {
    use super::*;

    /// Path to the element.
    pub const PATH: Tag = Tag::new(TagClass::Context, TagType::Primitive, 0);
    
    /// Element contents.
    pub const CONTENTS: Tag = Tag::new(TagClass::Context, TagType::Constructed, 1);
    
    /// Element children.
    pub const CHILDREN: Tag = Tag::new(TagClass::Context, TagType::Constructed, 2);
}
