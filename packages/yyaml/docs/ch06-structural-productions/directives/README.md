# 6.8. Directives

*Directives* are instructions to the YAML [processor](https://yaml.org/spec/1.2.2/#processes-and-models). This specification defines two directives, " `YAML` " and " `TAG` ", and *reserves* all other directives for future use. There is no way to define private directives. This is intentional.

Directives are a [presentation detail](https://yaml.org/spec/1.2.2/#presenting-the-serialization-tree) and must not be used to convey [content](https://yaml.org/spec/1.2.2/#nodes) information.

```
[82] l-directive ::=
  c-directive            # '%'
  (
      ns-yaml-directive
    | ns-tag-directive
    | ns-reserved-directive
  )
  s-l-comments
```

Each directive is specified on a separate non- [indented](https://yaml.org/spec/1.2.2/#indentation-spaces) line starting with the " `%` " indicator, followed by the directive name and a list of parameters. The semantics of these parameters depends on the specific directive. A YAML [processor](https://yaml.org/spec/1.2.2/#processes-and-models) should ignore unknown directives with an appropriate warning.

```
[83] ns-reserved-directive ::=
  ns-directive-name
  (
    s-separate-in-line
    ns-directive-parameter
  )*
```
```
[84] ns-directive-name ::=
  ns-char+
```
```
[85] ns-directive-parameter ::=
  ns-char+
```

**Example 6.13 Reserved Directives**

| ``` %FOO  bar baz # Should be ignored                # with a warning. --- "foo" ``` | ```json "foo" ``` |
| --- | --- |

**Legend:**

- `[ns-reserved-directive](https://yaml.org/spec/1.2.2/#rule-ns-reserved-directive)`
- `[ns-directive-name](https://yaml.org/spec/1.2.2/#rule-ns-directive-name)`
- `[ns-directive-parameter](https://yaml.org/spec/1.2.2/#rule-ns-directive-parameter)`

## Sections

- [6.8.1. YAML Directives](6.8.1-yaml-directives.md)
- [6.8.2. TAG Directives](6.8.2-tag-directives.md)
  - [6.8.2.1. Tag Handles](6.8.2.1-tag-handles.md)
  - [6.8.2.2. Tag Prefixes](6.8.2.2-tag-prefixes.md)
- [6.8.3. Reserved Directives](6.8.3-reserved-directives.md)