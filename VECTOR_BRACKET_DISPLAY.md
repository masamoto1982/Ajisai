# Vector/Tensor Bracket Display System

## Overview

Ajisai implements a visual distinction system for nested vectors and tensors using different bracket types based on nesting depth. This feature enhances code readability by making the nesting structure immediately apparent.

**Key Points:**
- The outermost brackets `[ ]` are always displayed
- Brackets cycle through 3 types based on depth: `[ ]` → `{ }` → `( )` → `[ ]` → ...
- Ajisai supports up to 4 dimensions; 5+ dimensions result in an error

## Design Philosophy

### Input vs Display Separation

Ajisai follows a clear separation between input parsing and display formatting:

1. **Input Normalization**: All bracket types (`[]`, `{}`, `()`) are treated identically during parsing
2. **Display Transformation**: Brackets are automatically converted based on nesting depth during display

This approach provides the following benefits:
- Users can input vectors using any bracket type they prefer
- Display is consistent and predictable regardless of input
- Visual nesting structure is always clear
- No cognitive overhead for choosing bracket types

### 4-Dimension Limit

Ajisai uses a [time, layer, row, col] dimension model:
- Maximum 4 dimensions are supported
- At 4 dimensions, brackets complete one full cycle
- Attempting to create 5+ dimensional tensors results in an error

## Technical Implementation

### Bracket Depth Mapping

The system uses modulo-3 arithmetic to cycle through three bracket styles:

```
Depth 0: [ ]  (Square brackets) - Outermost, always displayed
Depth 1: { }  (Curly braces)
Depth 2: ( )  (Round parentheses)
Depth 3: [ ]  (Cycles back to square brackets) - 4th dimension
```

Note: Since Ajisai is limited to 4 dimensions, the cycle completes exactly once at maximum depth.

### Examples

#### 1-Dimensional Tensor

**Input:**
```
[ 1 2 3 ]
```

**Display:**
```
[ 1 2 3 ]
```

#### 2-Dimensional Tensor (Matrix)

**Input:**
```
[[ 1 2 ] [ 3 4 ]]
```

**Display:**
```
[ { 1 2 } { 3 4 } ]
```

#### Mixed Input Brackets

**Input (using different bracket types):**
```
[{ 1 2 } ( 3 4 )]
```

**Display (normalized):**
```
[ { 1 2 } { 3 4 } ]
```

Note: All three inputs `[ 1 2 3 ]`, `{ 1 2 3 }`, and `( 1 2 3 )` are treated identically.

#### 3-Dimensional Tensor

**Input:**
```
[[[ 1 2 ] [ 3 4 ]] [[ 5 6 ] [ 7 8 ]]]
```

**Display:**
```
[ { ( 1 2 ) ( 3 4 ) } { ( 5 6 ) ( 7 8 ) } ]
```

#### 4-Dimensional Tensor (Maximum, Demonstrates Cycling)

**Input:**
```
[ 2 2 2 2 ] ZEROS
```

**Display:**
```
[ { ( [ 0 0 ] [ 0 0 ] ) ( [ 0 0 ] [ 0 0 ] ) } { ( [ 0 0 ] [ 0 0 ] ) ( [ 0 0 ] [ 0 0 ] ) } ]
```

Notice how depth 3 returns to square brackets, completing the cycle.

#### 5+ Dimensions (Error)

**Input:**
```
[ 2 2 2 2 2 ] ZEROS
```

**Result:**
```
Error: Ajisai supports up to 4 dimensions (time, layer, row, col), got 5
```

## Implementation Details

### Rust Backend (Parser)

Location: `rust/src/interpreter/mod.rs`

The `collect_vector` function:
1. Accepts any bracket type as input (`Token::VectorStart(BracketType)`)
2. Validates bracket matching (e.g., `[` must close with `]`)
3. **Normalizes all vectors to `BracketType::Square`** internally
4. Recursively processes nested vectors with the same normalization

Key code snippet:
```rust
Token::VectorStart(_) => {
    let (nested_values, _, consumed) = self.collect_vector(tokens, i)?;
    // Always use BracketType::Square regardless of input
    values.push(Value { val_type: ValueType::Vector(nested_values, BracketType::Square) });
    i += consumed;
}
```

### TypeScript Frontend (Display)

Location: `js/gui/display.ts`

The `formatValue` function:
1. Receives a `depth` parameter (starts at 0)
2. Calculates bracket type using `depth % 3`
3. Recursively formats nested vectors with `depth + 1`
4. Returns formatted string with appropriate brackets

Key code snippet:
```typescript
private formatValue(item: Value, depth: number = 0): string {
    // ...
    case 'vector': {
        const bracketIndex = depth % 3;
        let openBracket: string, closeBracket: string;

        switch (bracketIndex) {
            case 0: openBracket = '['; closeBracket = ']'; break;
            case 1: openBracket = '{'; closeBracket = '}'; break;
            case 2: openBracket = '('; closeBracket = ')'; break;
        }

        const elements = item.value.map((v: Value) =>
            this.formatValue(v, depth + 1)
        ).join(' ');

        return `${openBracket}${elements ? ' ' + elements + ' ' : ''}${closeBracket}`;
    }
}
```

## Consistency Across Operations

All vector operations that create new vectors use `BracketType::Square` internally:

- Vector concatenation (`CONCAT`)
- Vector mapping (`MAP`, `FILTER`, `REDUCE`)
- Vector construction from stack (`PACK`)
- Vector slicing and manipulation

This ensures that the display behavior is consistent regardless of how vectors are created.

## Benefits

1. **Visual Clarity**: Nesting structure is immediately apparent
2. **Reduced Cognitive Load**: No need to track or remember bracket types
3. **Consistency**: Display is predictable and uniform
4. **Flexibility**: Users can input using any bracket style they prefer
5. **Scalability**: Cycles through bracket types for arbitrary nesting depth

## Testing

See `test_nested_vector_brackets.ajisai` for comprehensive test cases demonstrating the bracket display system.

## Related Files

- `rust/src/interpreter/mod.rs` - Parser and bracket normalization
- `rust/src/types.rs` - `BracketType` enum definition
- `js/gui/display.ts` - Display formatting logic
- `rust/src/tokenizer.rs` - Token recognition for all bracket types
- `rust/src/wasm_api.rs` - WASM API for value conversion

## Commit History

- Commit `4c54882` (Oct 20, 2025): Introduced depth-based bracket display system
