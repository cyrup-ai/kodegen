# Word Boundary Matching

## Overview

Word boundary matching allows AI agents to search for patterns that match complete words or tokens rather than arbitrary substrings. This feature significantly reduces false positives and improves precision in code analysis and refactoring tasks.

## What Are Word Boundaries?

Word boundaries (`\b` in regex) are zero-width assertions that match at positions between word characters (`\w`: letters, digits, underscores) and non-word characters.

**Word Boundary Characters:**
- `.` (dot/period)
- `-` (hyphen)
- `_` (underscore when used as separator)
- `/` (forward slash)
- Spaces and line start/end

**Example:**
- `"test"` at a word boundary matches: `test`, `test()`, `test.log`, `test-suite`
- `"test"` at a word boundary does NOT match: `testing`, `attest`, `fastest`, `libtest`

## When to Use Word Boundaries

### 1. Function and Variable Name Matching
Avoid partial matches in compound names:

```rust
// Search: "add" with word_boundary=true
fn add(a: i32, b: i32) -> i32 { }    // ✓ Match
fn add_user() { }                     // ✗ No match (part of add_user)
let address = "123 Main St";         // ✗ No match (part of address)
```

### 2. Refactoring and Renaming
Ensure you only rename the exact identifier:

```javascript
// Search: "log" with word_boundary=true
console.log("message");      // ✓ Match
const logger = new Logger(); // ✗ No match
const catalog = getCatalog(); // ✗ No match
```

### 3. Keyword and Token Search
Find exact language keywords or tokens:

```python
# Search: "class" with word_boundary=true
class MyClass:        // ✓ Match
subclass = None       // ✗ No match
```

### 4. File Search Precision
Match exact filename components:

```bash
# Search: "lib" with word_boundary=true in filenames
lib.rs           // ✓ Match (lib is whole component)
lib-test.rs      // ✓ Match (lib before hyphen boundary)
libtest.rs       // ✗ No match (lib not at boundary)
test_lib.py      // ✓ Match (lib at end boundary)
```

## Usage

### Content Search

```json
{
  "pattern": "test",
  "search_type": "content",
  "word_boundary": true,
  "literal_search": false
}
```

**Result:** Matches `test` only when it appears as a complete word.

### File Search

```json
{
  "pattern": "lib",
  "search_type": "files",
  "word_boundary": true
}
```

**Result:** Matches filenames like `lib.rs`, `lib-test.rs`, but not `libtest.rs`.

## Literal vs. Regex Mode

### Literal Mode (`literal_search: true`)
Special regex characters are automatically escaped:

```json
{
  "pattern": "test.log",
  "literal_search": true,
  "word_boundary": true
}
```

**Matches:** `test.log` (dot is escaped: `\btest\.log\b`)  
**Does NOT match:** `testXlog` (would match without escaping)

### Regex Mode (`literal_search: false`)
Pattern is treated as regex and wrapped with boundaries:

```json
{
  "pattern": "test.*",
  "literal_search": false,
  "word_boundary": true
}
```

**Result:** `\b(?:test.*)\b` - matches `test`, `testing`, `tester` at word boundaries.

## Comparison: With vs. Without Word Boundary

### Example Code

```rust
fn test() { }              // Line 1
fn test_user() { }         // Line 2
fn testing() { }           // Line 3
let contest = "value";     // Line 4
// test comment             // Line 5
fn attest() { }            // Line 6
```

### Search: `"test"` WITHOUT word_boundary

**Matches:** All 6 lines (substring matching)

### Search: `"test"` WITH word_boundary

**Matches:** Only lines 1 and 5 (complete word matching)

**False Positives Eliminated:** 4 lines (67% reduction)

## Trade-offs and Considerations

### Advantages
- **Precision:** Reduces noise in search results
- **Refactoring Safety:** Ensures complete token matching
- **Token Efficiency:** Less context pollution for AI agents
- **Correctness:** More accurate code analysis

### Limitations

#### CamelCase Identifiers
Word boundaries don't exist within camelCase:

```javascript
someFunction  // "some" and "Function" are NOT separate words
```

Searching for `"Function"` with word boundary will NOT match `someFunction`.

#### Snake_case Works Naturally
Underscores create word boundaries:

```python
some_function  // "some" and "function" are separate words
```

Searching for `"function"` with word boundary WILL match `some_function`.

#### Unicode Considerations
The `\b` assertion works with Unicode word characters as defined by the regex engine.

## Implementation Details

### Content Search

Uses regex word boundary assertions (`\b`):

- **Literal patterns:** Escaped then wrapped: `\bpattern\b`
- **Regex patterns:** Wrapped with non-capturing group: `\b(?:pattern)\b`  
- **Existing boundaries:** Preserved (no double-wrapping)

### File Search

Uses custom boundary detection:

- Checks for separators: `.`, `-`, `_`, `/`
- Verifies pattern is surrounded by boundaries or string start/end
- Case-insensitive and smart-case modes supported

## Best Practices

1. **Use word boundaries for identifier searches**  
   Searching for function names, variable names, class names.

2. **Use substring mode for partial matches**  
   Searching within strings, comments, or compound identifiers.

3. **Combine with literal mode for special characters**  
   Searching for patterns with dots, parentheses, etc.

4. **Test with and without boundaries**  
   Compare result counts to verify you're getting expected precision.

## Examples

### Find All References to a Function

```json
{
  "pattern": "getUserData",
  "search_type": "content",
  "word_boundary": true,
  "case_mode": "sensitive"
}
```

Matches: `getUserData()`, `getUserData.call()`, `const data = getUserData`  
Ignores: `getUserDataFromCache`, `refreshUserData`

### Find Config Files

```json
{
  "pattern": "config",
  "search_type": "files",
  "word_boundary": true,
  "case_mode": "insensitive"
}
```

Matches: `config.json`, `app-config.js`, `test_config.py`  
Ignores: `configuration.txt`, `reconfig.sh`

### Find TODO Comments

```json
{
  "pattern": "TODO",
  "search_type": "content",
  "word_boundary": true
}
```

Matches: `// TODO: fix this`, `# TODO - refactor`  
Ignores: `TODOIST`, `TODO_LIST` (if searching for exact "TODO")

## See Also

- [Start Search API Documentation](../packages/filesystem/src/search/start_search.rs)
- [Boundary Mode Tests](../packages/filesystem/src/search/tests/boundary_tests.rs)
- [Integration Tests](../packages/filesystem/src/search/tests/integration_tests.rs)
