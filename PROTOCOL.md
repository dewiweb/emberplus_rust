# Ember+ Protocol Implementation Notes

Lessons learned from debugging the client against real Lawo hardware.

## S101 Framing Layer

### Message Structure
```
[BOF=0xFE] [SLOT] [MSG_TYPE] [CMD] [VERSION] [FLAGS?] [DTD?] [APP_COUNT?] [PAYLOAD] [CRC_LO] [CRC_HI] [EOF=0xFF]
```

### Critical Rules

1. **Message Type**: Always `0x0E` for both data and keep-alive (NOT `0x00`)

2. **Escaping**: Escape bytes `>= 0xF8` by XOR with `0x20` and prefix with `0xFD`
   ```rust
   if byte >= 0xF8 {
       output.push(0xFD);
       output.push(byte ^ 0x20);
   }
   ```

3. **CRC-16-CCITT**:
   - Use **reflected** polynomial table
   - Calculate on **escaped** data (unescape while computing)
   - Final CRC is **inverted** (`!crc`)
   - CRC bytes themselves must be escaped if `>= 0xF8`

4. **Keep-alive frame**: `FE 00 0E 01 01 [CRC] FF`

## BER/Glow Encoding

### Root Structure
```
60 xx          -- GlowRoot (APPLICATION 0, constructed)
  6b xx        -- RootElementCollection (APPLICATION 11)
    [elements]
```

### Node Contents - CRITICAL
Node contents must be wrapped in a **SET (0x31)**, and strings use **UTF8String (0x0C)**:

```
a1 xx          -- context[1] (contents)
  31 xx        -- SET wrapper (REQUIRED!)
    a0 xx      -- context[0] (identifier)
      0c xx    -- UTF8String (NOT raw bytes!)
        [string bytes]
```

**Wrong** (causes connection close):
```
a1 09 80 07 "facades"   -- context[1] with primitive string
```

**Correct**:
```
a1 0d 31 0b a0 09 0c 07 "facades"   -- SET > context[0] > UTF8String
```

### QualifiedNode (APPLICATION 10)
Must be wrapped in **context[0]** (NumberedTreeNode):

```
a0 xx          -- context[0] NumberedTreeNode wrapper (REQUIRED!)
  6a xx        -- QualifiedNode (APPLICATION 10)
    a0 xx      -- context[0] path
      0d xx    -- RELATIVE-OID
        [path bytes]
    a1 xx      -- context[1] contents (with SET + UTF8String)
    a2 xx      -- context[2] children (commands)
```

### Path Encoding (RELATIVE-OID)
- Tag: `0x0D` (Universal Primitive 13)
- Each component encoded with 7-bit chunks, MSB set for continuation

```rust
// Path [5, 1, 1] encodes as: 0d 03 05 01 01
// Path [5] encodes as: 0d 01 05
```

### Commands
Commands are wrapped in APPLICATION tags inside context[2]:

```
a2 xx          -- context[2] children
  64 xx        -- ElementCollection (APPLICATION 4)
    a0 xx      -- context[0] NumberedTreeNode
      62 xx    -- Command (APPLICATION 2)
        a0 xx  -- context[0] command number
          02 01 20  -- INTEGER 32 (GetDirectory)
```

Command numbers:
- `32` (0x20) = GetDirectory
- `33` (0x21) = Subscribe
- `34` (0x22) = Unsubscribe
- `35` (0x23) = Invoke

### Invoke Command (APPLICATION 22)
```
a2 xx          -- context[2] children
  64 xx        -- ElementCollection (APPLICATION 4)
    a0 xx      -- context[0]
      76 xx    -- Invocation (APPLICATION 22)
        a0 xx  -- context[0] invocation ID
          02 xx [id]
        a1 xx  -- context[1] arguments (optional)
```

## Server Implementation Checklist

### Receiving from Client
- [ ] Decode S101 with correct CRC verification
- [ ] Handle NumberedTreeNode wrapper (context[0])
- [ ] Parse QualifiedNode paths as RELATIVE-OID
- [ ] Handle commands inside context[2] > ElementCollection

### Sending to Client
- [ ] Wrap all elements in NumberedTreeNode (context[0])
- [ ] Use SET wrapper for node/parameter contents
- [ ] Use UTF8String for all string fields
- [ ] Encode paths as RELATIVE-OID
- [ ] Calculate CRC correctly (reflected, inverted)

### Response Format
When responding to GetDirectory, send children with full contents:

```
60 xx          -- Root
  6b xx        -- RootElementCollection
    a0 xx      -- NumberedTreeNode
      6a xx    -- QualifiedNode
        a0 xx  -- path
        a1 xx  -- contents (SET wrapped)
        a2 xx  -- children
          64 xx  -- ElementCollection
            [child nodes with their contents]
```

## Testing Against Reference

Compare frames with `emberplus-connection` (Node.js):
```javascript
const { EmberClient } = require('emberplus-connection');
// Capture frames to compare byte-by-byte
```
