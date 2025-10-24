# Chapter 2. Language Overview

This section provides a quick glimpse into the expressive power of YAML. It is not expected that the first-time reader grok all of the examples. Rather, these selections are used as motivation for the remainder of the specification.

YAML's [block collections](https://yaml.org/spec/1.2.2/#block-collection-styles) use [indentation](https://yaml.org/spec/1.2.2/#indentation-spaces) for scope and begin each entry on its own line.[Block sequences](https://yaml.org/spec/1.2.2/#block-sequences) indicate each entry with a dash and space (" `- ` ").[Mappings](https://yaml.org/spec/1.2.2/#mapping) use a colon and space ("`: `") to mark each [key/value pair](https://yaml.org/spec/1.2.2/#mapping).[Comments](https://yaml.org/spec/1.2.2/#comments) begin with an octothorpe (also called a "hash", "sharp", "pound" or "number sign" - " `#` ").

## Sections

- [2.2. Structures](2.2-structures.md)
- [2.3. Scalars](2.3-scalars.md)
- [2.4. Tags](2.4-tags.md)
- [2.5. Full Length Example](2.5-full-length-example.md)

## Basic Examples

**Example 2.1 Sequence of Scalars (ball players)**

```
- Mark McGwire
- Sammy Sosa
- Ken Griffey
```

**Example 2.2 Mapping Scalars to Scalars (player statistics)**

```
hr:  65    # Home runs
avg: 0.278 # Batting average
rbi: 147   # Runs Batted In
```

**Example 2.3 Mapping Scalars to Sequences (ball clubs in each league)**

```
american:
- Boston Red Sox
- Detroit Tigers
- New York Yankees
national:
- New York Mets
- Chicago Cubs
- Atlanta Braves
```

**Example 2.4 Sequence of Mappings (players' statistics)**

```
-
  name: Mark McGwire
  hr:   65
  avg:  0.278
-
  name: Sammy Sosa
  hr:   63
  avg:  0.288
```

YAML also has [flow styles](https://yaml.org/spec/1.2.2/#flow-style-productions), using explicit [indicators](https://yaml.org/spec/1.2.2/#indicator-characters) rather than [indentation](https://yaml.org/spec/1.2.2/#indentation-spaces) to denote scope. The [flow sequence](https://yaml.org/spec/1.2.2/#flow-sequences) is written as a [comma](https://yaml.org/spec/1.2.2/#flow-collection-styles) separated list within [square](https://yaml.org/spec/1.2.2/#flow-sequences) [brackets](https://yaml.org/spec/1.2.2/#flow-sequences). In a similar manner, the [flow mapping](https://yaml.org/spec/1.2.2/#flow-mappings) uses [curly](https://yaml.org/spec/1.2.2/#flow-mappings) [braces](https://yaml.org/spec/1.2.2/#flow-mappings).

**Example 2.5 Sequence of Sequences**

```
- [name        , hr, avg  ]
- [Mark McGwire, 65, 0.278]
- [Sammy Sosa  , 63, 0.288]
```

**Example 2.6 Mapping of Mappings**

```
Mark McGwire: {hr: 65, avg: 0.278}
Sammy Sosa: {
    hr: 63,
    avg: 0.288,
 }
```