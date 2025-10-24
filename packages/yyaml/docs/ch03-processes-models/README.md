# Chapter 3. Processes and Models

YAML is both a text format and a method for [presenting](https://yaml.org/spec/1.2.2/#presenting-the-serialization-tree) any [native data structure](https://yaml.org/spec/1.2.2/#representing-native-data-structures) in this format. Therefore, this specification defines two concepts: a class of data objects called YAML [representations](https://yaml.org/spec/1.2.2/#representation-graph) and a syntax for [presenting](https://yaml.org/spec/1.2.2/#presenting-the-serialization-tree) YAML [representations](https://yaml.org/spec/1.2.2/#representation-graph) as a series of characters, called a YAML [stream](https://yaml.org/spec/1.2.2/#streams).

A YAML *processor* is a tool for converting information between these complementary views. It is assumed that a YAML processor does its work on behalf of another module, called an *application*. This chapter describes the information structures a YAML processor must provide to or obtain from the application.

YAML information is used in two ways: for machine processing and for human consumption. The challenge of reconciling these two perspectives is best done in three distinct translation stages: [representation](https://yaml.org/spec/1.2.2/#representation-graph), [serialization](https://yaml.org/spec/1.2.2/#serialization-tree) and [presentation](https://yaml.org/spec/1.2.2/#presentation-stream).[Representation](https://yaml.org/spec/1.2.2/#representation-graph) addresses how YAML views [native data structures](https://yaml.org/spec/1.2.2/#representing-native-data-structures) to achieve portability between programming environments.[Serialization](https://yaml.org/spec/1.2.2/#serialization-tree) concerns itself with turning a YAML [representation](https://yaml.org/spec/1.2.2/#representation-graph) into a serial form, that is, a form with sequential access constraints.[Presentation](https://yaml.org/spec/1.2.2/#presentation-stream) deals with the formatting of a YAML [serialization](https://yaml.org/spec/1.2.2/#serialization-tree) as a series of characters in a human-friendly manner.

## Sections

- [3.1. Processes](3.1-processes.md)
  - [3.1.1. Dump](3.1.1-dump.md)
  - [3.1.2. Load](3.1.2-load.md)
- [3.2. Information Models](information-models/)
  - [3.2.1. Representation Graph](information-models/3.2.1-representation-graph.md)
    - [3.2.1.1. Nodes](information-models/3.2.1.1-nodes.md)
    - [3.2.1.2. Tags](information-models/3.2.1.2-tags.md) 
    - [3.2.1.3. Node Comparison](information-models/3.2.1.3-node-comparison.md)
  - [3.2.2. Serialization Tree](serialization-tree/)
    - [3.2.2.1. Mapping Key Order](serialization-tree/3.2.2.1-mapping-key-order.md)
    - [3.2.2.2. Anchors and Aliases](serialization-tree/3.2.2.2-anchors-aliases.md)
  - [3.2.3. Presentation Stream](presentation-stream/)
    - [3.2.3.1. Node Styles](presentation-stream/3.2.3.1-node-styles.md)
    - [3.2.3.2. Scalar Formats](presentation-stream/3.2.3.2-scalar-formats.md)
    - [3.2.3.4. Directives](presentation-stream/3.2.3.4-directives.md)
- [3.3. Loading Failure Points](3.3-loading-failure-points.md)
  - [3.3.1. Well-Formed Streams and Identified Aliases](3.3.1-well-formed-streams.md)
  - [3.3.2. Resolved Tags](3.3.2-resolved-tags.md)
  - [3.3.3. Recognized and Valid Tags](3.3.3-recognized-valid-tags.md)
  - [3.3.4. Available Tags](3.3.4-available-tags.md)

## Processing Overview

**Figure 3.1. Processing Overview**

![Processing Overview](https://yaml.org/spec/1.2.2/img/overview2.svg)

Translating between [native data structures](https://yaml.org/spec/1.2.2/#representing-native-data-structures) and a character [stream](https://yaml.org/spec/1.2.2/#streams) is done in several logically distinct stages, each with a well defined input and output data model.