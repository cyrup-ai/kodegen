# Chapter 10. Recommended Schemas

A sequence of bytes is a *well-formed stream* if, taken as a whole, it complies with the above `l-yaml-stream` production.

A YAML *schema* is a combination of a set of [tags](https://yaml.org/spec/1.2.2/#tags) and a mechanism for [resolving](https://yaml.org/spec/1.2.2/#resolved-tags) [non-specific tags](https://yaml.org/spec/1.2.2/#resolved-tags).

## Sections

- [10.1. Failsafe Schema](10.1-failsafe-schema.md)
  - [10.1.1. Tags](failsafe/10.1.1-tags.md)
    - [10.1.1.1. Generic Mapping](failsafe/10.1.1.1-generic-mapping.md)
    - [10.1.1.2. Generic Sequence](failsafe/10.1.1.2-generic-sequence.md)
    - [10.1.1.3. Generic String](failsafe/10.1.1.3-generic-string.md)
  - [10.1.2. Tag Resolution](failsafe/10.1.2-tag-resolution.md)
- [10.2. JSON Schema](10.2-json-schema.md)
  - [10.2.1. Tags](json/10.2.1-tags.md)
    - [10.2.1.1. Null](json/10.2.1.1-null.md)
    - [10.2.1.2. Boolean](json/10.2.1.2-boolean.md)
    - [10.2.1.3. Integer](json/10.2.1.3-integer.md)
    - [10.2.1.4. Floating Point](json/10.2.1.4-floating-point.md)
  - [10.2.2. Tag Resolution](json/10.2.2-tag-resolution.md)
- [10.3. Core Schema](10.3-core-schema.md)
  - [10.3.1. Tags](core/10.3.1-tags.md)
  - [10.3.2. Tag Resolution](core/10.3.2-tag-resolution.md)
- [10.4. Other Schemas](10.4-other-schemas.md)