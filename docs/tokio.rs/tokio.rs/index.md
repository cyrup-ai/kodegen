![](/img/icons/bytes.svg)

![](/img/icons/hyper.svg)

![](/img/icons/mio.svg)

![](/img/icons/runtime.svg)

![](/img/icons/runtime.svg)

![](/img/icons/tonic.svg)

![](/img/icons/tower.svg)

![](/img/icons/tracing.svg)

# Build reliable network applications without compromising speed.

Tokio is an asynchronous runtime for the Rust programming language. It provides the building blocks needed for writing network applications. It gives the flexibility to target a wide range of systems, from large servers with dozens of cores to small embedded devices.
- ---------

[Get Started](tokio/tutorial/index.html)

# Built by the community, for the community.

# Reliable

Tokio's APIs are memory-safe, thread-safe, and misuse-resistant. This helps prevent common bugs, such as unbounded queues, buffer overflows, and task starvation.
- ---------

# Fast

Building on top of Rust, Tokio provides a multi-threaded, work-stealing scheduler. Applications can process hundreds of thousands of requests per second with minimal overhead.
- ---------

# Easy

`async`/`await` reduces the complexity of writing asynchronous applications. Paired with Tokio's utilities and vibrant ecosystem, writing applications is a breeze.
- ---------

# Flexible

The needs of a server application differ from that of an embedded device. Although Tokio comes with defaults that work well out of the box, it also provides the knobs needed to fine tune to different cases.
- ---------

# ![The stack](/img/icons/tokio.svg)The stack

Applications aren't built in a vacuum. The Tokio stack includes everything needed to ship to production, fast.
- ---------

# ![Runtime](/img/icons/runtime.svg)Runtime

Including I/O, timer, filesystem, synchronization, and scheduling facilities, the Tokio runtime is the foundation of asynchronous applications.
- ---------

[Learn more ➔](tokio/tutorial/index.html)

# ![Hyper](/img/icons/hyper.svg)Hyper

An HTTP client and server library supporting both the HTTP 1 and 2 protocols.
- ---------

[Learn more ➔](https://github.com/hyperium/hyper)

# ![Tonic](/img/icons/tonic.svg)Tonic

A boilerplate-free gRPC client and server library. The easiest way to expose and consume an API over the network.
- ---------

[Learn more ➔](https://github.com/hyperium/tonic)

# ![Tower](/img/icons/tower.svg)Tower

Modular components for building reliable clients and servers. Includes retry, load-balancing, filtering, request-limiting facilities, and more.
- ---------

[Learn more ➔](https://github.com/tower-rs/tower)

# ![Mio](/img/icons/mio.svg)Mio

Minimal portable API on top of the operating-system's evented I/O API.
- ---------

[Learn more ➔](https://github.com/tokio-rs/mio)

# ![Tracing](/img/icons/tracing.svg)Tracing

Unified insight into the application and libraries. Provides structured, event-based, data collection and logging.
- ---------

[Learn more ➔](https://github.com/tokio-rs/tracing)

# ![Bytes](/img/icons/bytes.svg)Bytes

At the core, networking applications manipulate byte streams. Bytes provides a rich set of utilities for manipulating byte arrays.
- ---------

[Learn more ➔](https://github.com/tokio-rs/bytes)

![](/img/stack-runtime.svg)![](/img/stack-hyper.svg)![](/img/stack-tonic.svg)![](/img/stack-tower.svg)![](/img/stack-mio.svg)![](/img/stack-tracing.svg)![](/img/stack-bytes.svg)![](/img/stack-lines.svg)