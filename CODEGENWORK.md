# Rust ↔ TypeScript Codegen: Full Bidirectional Production Plan

## Overview

Full bidirectional support means TypeScript types must:
1. **Match Rust serialization exactly** - what serde outputs, TS must accept
2. **Produce valid Rust input** - what TS sends, serde must deserialize

---

## Architecture Context

### Codegen Files

| File | Purpose |
|------|---------|
| `packages/server/internal/codegen/src/lib.rs` | Main codegen - ExportedType enum, parse_type, to_typescript, generators |
| `packages/server/internal/macros/src/metadata.rs` | TypeMetadata enum (mirrors ExportedType) |
| `packages/server/internal/macros/src/types.rs` | Type parsing for macros |
| `packages/server/internal/macros/src/lib.rs` | `#[export]` proc macro |

### RPC Flow

```
TypeScript                          Rust
    │                                 │
    ├─── rpcCall() ──────────────────►│ deserialize_message()
    │    (MessagePack/JSON)           │ (rmp_serde / serde_json)
    │                                 │
    │◄────────────────────────────────┤ serialize_message()
    │    (MessagePack/JSON)           │ (to_vec_named)
```

**Transport:** Unix Domain Socket with length-prefixed framing `[4-byte BE length][payload]`

**Encoding:** MessagePack (default) or JSON - auto-detected by first byte

### Serde Patterns Used

| Attribute | Usage | Effect |
|-----------|-------|--------|
| `#[serde(rename = "camelCase")]` | Field renaming | `created_at` → `createdAt` |
| `#[serde(tag = "type")]` | Tagged enums | `{ type: "variant", ...data }` |
| `#[serde(rename_all = "snake_case")]` | Enum variants | Variant names lowercase |
| `#[serde(skip_serializing_if = "Option::is_none")]` | Optional fields | Omit field if None |
| `#[serde(default)]` | Default values | Use default on missing |

---

## Current State

### What Works
- Basic primitives (String, bool, numbers)
- `Option<T>` → `T | null`
- `Vec<T>` → `T[]`
- `HashMap<K, V>` → `Record<K, V>`
- `Result<T, E>` → `T | E` union
- `#[serde(rename)]` on fields

### What's Broken

| Issue | Current Behavior | Correct Behavior |
|-------|-----------------|------------------|
| Enums | Becomes undefined `Custom` type | Tagged union type |
| Box/Arc/Rc/Cow | `Box<User>` stays `Box<User>` | Unwrap to `User` |
| Tuples | `Tuple2<string, number>` | `[string, number]` |
| Slices/Arrays | `unknown` | `T[]` |
| `skip_serializing_if` | Field always required | Field becomes optional `?:` |
| `#[serde(default)]` | Not tracked | TypeScript needs to know |
| Tagged enums | Not supported | `{ type: "variant", ...data }` |

---

## Implementation Plan

### Phase 1: Core Type Fixes

#### 1.1 Wrapper Type Unwrapping

**Files:** `codegen/src/lib.rs:702-760`, `macros/src/types.rs:42-96`

```rust
// In parse_type, add before custom type fallback:
"Box" | "Arc" | "Rc" | "Cow" | "RefCell" | "Cell" | "Mutex" | "RwLock" => {
    generics.into_iter().next().unwrap_or(ExportedType::Unit)
}
"PhantomData" => ExportedType::Unit,
```

**Test:**
- `Box<String>` → `string`
- `Arc<User>` → `User`
- `PhantomData<T>` → omit from output

---

#### 1.2 Tuple Types

**Add to ExportedType enum (lib.rs:32-62):**
```rust
Tuple(Vec<ExportedType>),
```

**Add to_typescript (lib.rs:83-130):**
```rust
ExportedType::Tuple(elements) => {
    let types = elements.iter().map(|e| e.to_typescript()).collect::<Vec<_>>().join(", ");
    format!("[{}]", types)
}
```

**Update parse_type (lib.rs:763):**
```rust
Type::Tuple(tuple) => {
    if tuple.elems.is_empty() {
        ExportedType::Unit
    } else {
        ExportedType::Tuple(tuple.elems.iter().map(parse_type).collect())
    }
}
```

**Test:**
- `(String, i32)` → `[string, number]`
- `(String, i32, bool)` → `[string, number, boolean]`
- `()` → `void`

---

#### 1.3 Slice and Array Types

**Add to ExportedType enum:**
```rust
Array(Box<ExportedType>),  // Both slices and arrays become arrays
```

**Add to_typescript:**
```rust
ExportedType::Array(inner) => format!("{}[]", inner.to_typescript()),
```

**Update parse_type (lib.rs:764-768):**
```rust
Type::Slice(slice) => ExportedType::Array(Box::new(parse_type(&slice.elem))),
Type::Array(array) => ExportedType::Array(Box::new(parse_type(&array.elem))),
```

**Test:**
- `&[u8]` → `number[]`
- `[String; 10]` → `string[]`

---

### Phase 2: Serde Attribute Support

#### 2.1 Track Field Optionality

**Modify StructField (lib.rs:72-79):**
```rust
pub struct StructField {
    pub name: String,
    pub ty: ExportedType,
    pub ts_name: Option<String>,
    pub optional: bool,                    // From Option<T>
    pub skip_serializing_if: bool,         // NEW: From serde attribute
    pub has_default: bool,                 // NEW: From #[serde(default)]
}
```

**Add extraction function:**
```rust
fn extract_serde_field_attrs(attrs: &[Attribute]) -> (Option<String>, bool, bool) {
    let mut rename = None;
    let mut skip_serializing_if = false;
    let mut has_default = false;

    for attr in attrs {
        if attr.path().is_ident("serde") {
            let tokens = attr.meta.to_token_stream().to_string();
            if tokens.contains("skip_serializing_if") {
                skip_serializing_if = true;
            }
            if tokens.contains("default") {
                has_default = true;
            }
            // extract rename...
        }
    }
    (rename, skip_serializing_if, has_default)
}
```

**Update TypeScript interface generation (lib.rs:491-499):**
```rust
// Field is optional if: Option<T> OR skip_serializing_if OR has_default
let is_optional = field.optional || field.skip_serializing_if || field.has_default;
if is_optional {
    output.push_str(&format!("  {}?: {};\n", ts_name, ts_type));
} else {
    output.push_str(&format!("  {}: {};\n", ts_name, ts_type));
}
```

**Test:**
```rust
#[derive(Serialize)]
pub struct Config {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub enabled: bool,
}
```
→
```typescript
interface Config {
    name: string;
    description?: string | null;
    enabled?: boolean;
}
```

---

#### 2.2 Support `#[serde(flatten)]`

**Add to StructField:**
```rust
pub flatten: bool,
```

**Generate TypeScript intersection type:**
```typescript
type Parent = BaseFields & FlattenedType;
```

---

### Phase 3: Enum Support (Critical)

#### 3.1 Add Enum Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EnumVariant {
    pub name: String,
    pub kind: EnumVariantKind,
    pub serde_rename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EnumVariantKind {
    Unit,                                    // Foo
    Newtype(Box<ExportedType>),             // Foo(String)
    Tuple(Vec<ExportedType>),               // Foo(String, i32)
    Struct { fields: Vec<StructField> },    // Foo { bar: String }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EnumRepresentation {
    ExternallyTagged,                              // Default: { "Variant": data }
    InternallyTagged { tag: String },              // #[serde(tag = "type")]
    AdjacentlyTagged { tag: String, content: String }, // #[serde(tag = "t", content = "c")]
    Untagged,                                      // #[serde(untagged)]
}

// Add to ExportedType enum
Enum {
    name: String,
    representation: EnumRepresentation,
    variants: Vec<EnumVariant>,
},
```

#### 3.2 Parse Enum Representation

```rust
fn parse_enum_representation(attrs: &[Attribute]) -> EnumRepresentation {
    for attr in attrs {
        if attr.path().is_ident("serde") {
            let tokens = attr.meta.to_token_stream().to_string();

            if let Some(tag) = extract_attr_value(&tokens, "tag") {
                if let Some(content) = extract_attr_value(&tokens, "content") {
                    return EnumRepresentation::AdjacentlyTagged { tag, content };
                }
                return EnumRepresentation::InternallyTagged { tag };
            }

            if tokens.contains("untagged") {
                return EnumRepresentation::Untagged;
            }
        }
    }
    EnumRepresentation::ExternallyTagged
}
```

#### 3.3 TypeScript Generation for Enums

```rust
ExportedType::Enum { name, representation, variants } => {
    match representation {
        // Default: { "Variant": data } or "Variant" for unit
        EnumRepresentation::ExternallyTagged => {
            variants.iter().map(|v| {
                let variant_name = v.serde_rename.as_ref().unwrap_or(&v.name);
                match &v.kind {
                    EnumVariantKind::Unit => format!("\"{}\"", variant_name),
                    EnumVariantKind::Newtype(inner) => {
                        format!("{{ {}: {} }}", variant_name, inner.to_typescript())
                    }
                    EnumVariantKind::Tuple(types) => {
                        let tuple = types.iter()
                            .map(|t| t.to_typescript())
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{{ {}: [{}] }}", variant_name, tuple)
                    }
                    EnumVariantKind::Struct { fields } => {
                        let fields_ts = fields.iter()
                            .map(|f| format!("{}: {}", f.ts_name.as_ref().unwrap_or(&f.name), f.ty.to_typescript()))
                            .collect::<Vec<_>>()
                            .join("; ");
                        format!("{{ {}: {{ {} }} }}", variant_name, fields_ts)
                    }
                }
            }).collect::<Vec<_>>().join(" | ")
        }

        // #[serde(tag = "type")]: { type: "variant", ...data }
        EnumRepresentation::InternallyTagged { tag } => {
            variants.iter().map(|v| {
                let variant_name = v.serde_rename.as_ref().unwrap_or(&v.name);
                match &v.kind {
                    EnumVariantKind::Unit => {
                        format!("{{ {}: \"{}\" }}", tag, variant_name)
                    }
                    EnumVariantKind::Struct { fields } => {
                        let fields_ts = fields.iter()
                            .map(|f| format!("{}: {}", f.ts_name.as_ref().unwrap_or(&f.name), f.ty.to_typescript()))
                            .collect::<Vec<_>>()
                            .join("; ");
                        format!("{{ {}: \"{}\"; {} }}", tag, variant_name, fields_ts)
                    }
                    _ => format!("{{ {}: \"{}\" }}", tag, variant_name)
                }
            }).collect::<Vec<_>>().join(" | ")
        }

        // #[serde(tag = "t", content = "c")]: { t: "variant", c: data }
        EnumRepresentation::AdjacentlyTagged { tag, content } => {
            variants.iter().map(|v| {
                let variant_name = v.serde_rename.as_ref().unwrap_or(&v.name);
                match &v.kind {
                    EnumVariantKind::Unit => {
                        format!("{{ {}: \"{}\" }}", tag, variant_name)
                    }
                    EnumVariantKind::Newtype(inner) => {
                        format!("{{ {}: \"{}\"; {}: {} }}", tag, variant_name, content, inner.to_typescript())
                    }
                    EnumVariantKind::Tuple(types) => {
                        let tuple = types.iter()
                            .map(|t| t.to_typescript())
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{{ {}: \"{}\"; {}: [{}] }}", tag, variant_name, content, tuple)
                    }
                    EnumVariantKind::Struct { fields } => {
                        let fields_ts = fields.iter()
                            .map(|f| format!("{}: {}", f.ts_name.as_ref().unwrap_or(&f.name), f.ty.to_typescript()))
                            .collect::<Vec<_>>()
                            .join("; ");
                        format!("{{ {}: \"{}\"; {}: {{ {} }} }}", tag, variant_name, content, fields_ts)
                    }
                }
            }).collect::<Vec<_>>().join(" | ")
        }

        // #[serde(untagged)]: just the data, no discriminator
        EnumRepresentation::Untagged => {
            variants.iter().map(|v| {
                match &v.kind {
                    EnumVariantKind::Unit => "null".to_string(),
                    EnumVariantKind::Newtype(inner) => inner.to_typescript(),
                    EnumVariantKind::Tuple(types) => {
                        let tuple = types.iter()
                            .map(|t| t.to_typescript())
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("[{}]", tuple)
                    }
                    EnumVariantKind::Struct { fields } => {
                        let fields_ts = fields.iter()
                            .map(|f| format!("{}: {}", f.ts_name.as_ref().unwrap_or(&f.name), f.ty.to_typescript()))
                            .collect::<Vec<_>>()
                            .join("; ");
                        format!("{{ {} }}", fields_ts)
                    }
                }
            }).collect::<Vec<_>>().join(" | ")
        }
    }
}
```

#### 3.4 Enum Test Cases

```rust
// Externally tagged (default)
#[derive(Serialize)]
pub enum Message {
    Text(String),
    Image { url: String, width: u32 },
    Ping,
}
// → { Text: string } | { Image: { url: string; width: number } } | "Ping"

// Internally tagged
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum IpcMessage {
    InvokeHandler { handler_id: String },
    HealthCheck,
}
// → { type: "InvokeHandler"; handler_id: string } | { type: "HealthCheck" }

// With rename_all
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    UserCreated { user_id: String },
    OrderPlaced { order_id: String },
}
// → { type: "user_created"; user_id: string } | { type: "order_placed"; order_id: string }
```

---

### Phase 4: Safety & Quality

#### 4.1 Cycle Detection

```rust
fn collect_custom_types_with_cycle_detection(
    ty: &ExportedType,
    types: &mut HashSet<String>,
    visiting: &mut HashSet<String>,
) {
    match ty {
        ExportedType::Custom { name, generics } => {
            if visiting.contains(name) {
                return; // Cycle detected
            }
            visiting.insert(name.clone());
            types.insert(name.clone());
            for g in generics {
                collect_custom_types_with_cycle_detection(g, types, visiting);
            }
            visiting.remove(name);
        }
        // Handle all other variants recursively...
    }
}
```

#### 4.2 Warnings for Unsupported Patterns

```rust
// In parse_type fallback
other => {
    eprintln!("Warning: Unsupported type {:?} - falling back to 'unknown'", other);
    ExportedType::Custom {
        name: "unknown".to_string(),
        generics: vec![],
    }
}
```

#### 4.3 Test Suite

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_wrapper_unwrapping() {
        let ty: Type = parse_quote!(Box<String>);
        assert_eq!(parse_type(&ty).to_typescript(), "string");

        let ty: Type = parse_quote!(Arc<User>);
        assert_eq!(parse_type(&ty).to_typescript(), "User");
    }

    #[test]
    fn test_tuples() {
        let ty: Type = parse_quote!((String, i32));
        assert_eq!(parse_type(&ty).to_typescript(), "[string, number]");
    }

    #[test]
    fn test_slices() {
        let ty: Type = parse_quote!(&[u8]);
        assert_eq!(parse_type(&ty).to_typescript(), "number[]");
    }
}
```

---

### Phase 5: Mirror Changes in Macros

All changes in `codegen/src/lib.rs` must be mirrored in:
- `macros/src/metadata.rs` - TypeMetadata enum
- `macros/src/types.rs` - parse_type function

This ensures compile-time and codegen-time parsing produce identical results.

---

## Implementation Order

1. **Wrapper types** - Simplest, high impact
2. **Tuples** - Small change
3. **Slices/Arrays** - Small change
4. **StructField serde attrs** - skip_serializing_if, default
5. **Cycle detection** - Safety
6. **Enum parsing** - Complex, save for last
7. **Enum TypeScript generation** - All 4 representations
8. **Tests** - Throughout

---

## Out of Scope

These require architectural changes beyond codegen:

- Generic functions (rejected at compile time by design)
- Trait objects (`dyn Trait`)
- Function pointers
- Associated types
- Const generics
- Where clauses
- Custom serializers/deserializers (`#[serde(with = "...")]`)

---

## Validation Strategy

After implementation, run codegen on zaptest and verify:

1. All existing types still work
2. New types generate correct TypeScript
3. Compile TypeScript to verify no errors
4. Round-trip test: TS → JSON → Rust → JSON → TS matches

---

## Key Line References

| Location | Purpose |
|----------|---------|
| `codegen/src/lib.rs:32-62` | ExportedType enum definition |
| `codegen/src/lib.rs:72-79` | StructField definition |
| `codegen/src/lib.rs:83-130` | to_typescript() implementation |
| `codegen/src/lib.rs:152-173` | collect_custom_types() |
| `codegen/src/lib.rs:475-506` | generate_typescript_interfaces() |
| `codegen/src/lib.rs:521-544` | extract_serde_rename() |
| `codegen/src/lib.rs:546-589` | parse_struct() |
| `codegen/src/lib.rs:671-769` | parse_type() |
| `macros/src/metadata.rs:34-71` | TypeMetadata enum |
| `macros/src/metadata.rs:73-128` | TypeMetadata::to_typescript() |
| `macros/src/types.rs:8-24` | parse_type() entry |
| `macros/src/types.rs:39-97` | parse_path_segment() |
