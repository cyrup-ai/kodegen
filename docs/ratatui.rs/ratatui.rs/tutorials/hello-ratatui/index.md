# Hello Ratatui

This tutorial will lead you through creating a simple “Hello World” TUI app that displays some text
in the middle of the screen and waits for the user to press any key to exit. It demonstrates the
tasks that any application developed with Ratatui needs to undertake.

We assume you have a basic understanding of the terminal, and have a text editor or IDE. If you
don’t have a preference, [VSCode](https://code.visualstudio.com/) with [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) makes a good default choice.

Pre-requisites
- ---------

[Section titled “Pre-requisites”](#pre-requisites)

## # Install Rust

[Section titled “Install Rust”](#install-rust)

First install Rust if it is not already installed. See the [Installation](https://doc.rust-lang.org/book/ch01-01-installation.html) section of the official
Rust Book for more information. Most people use `rustup`, a command line tool for managing Rust
versions and associated tools. Ratatui requires at least Rust 1.74, but it’s generally a good idea
to work with the latest stable version if you can. Once you’ve installed Rust, verify it’s installed
by running:

```
rustc --version
```

You should see output similar to the following (the exact version, date and commit hash will vary):

```
rustc 1.83.0 (90b35a623 2024-11-26)
```

## # Install Cargo generate

[Section titled “Install Cargo generate”](#install-cargo-generate)

Ratatui has a few templates that make it easy to get started with a new project. [Cargo generate](https://cargo-generate.github.io/cargo-generate/) is
a developer tool to help you get up and running quickly with a new Rust project by leveraging a
pre-existing git repository as a template. We will use it to create a new Ratatui project.

Install `cargo-generate` by running the following command (or see the [installation instructions](https://cargo-generate.github.io/cargo-generate/installation.html)for other approaches to installing cargo-generate.)

```
cargo install cargo-generate
```

Create a New Project
- ---------

[Section titled “Create a New Project”](#create-a-new-project)

Let’s create a new Rust project. In the terminal, navigate to a folder where you will store your
projects and run the following command to generate a new app using the simple ratatui template. (You
can find more information about this template in the [Hello World Template README](https://github.com/ratatui/templates/blob/main/hello-world/README.md))

```
cargo generate ratatui/templates simple
```

You will be prompted for a project name to use. Enter `hello-ratatui`.

```
$ cargo generate ratatui/templates⚠️   Favorite `ratatui/templates` not found in config, using it as a git repository: https://github.com/ratatui/templates.git✔ 🤷   Which sub-template should be expanded? · hello-world🤷   Project Name: hello-ratatui🔧   Destination: /Users/joshka/local/ratatui-website/code/tutorials/hello-ratatui ...🔧   project-name: hello-ratatui ...🔧   Generating template ...🔧   Moving generated files into: `/Users/joshka/local/ratatui-website/code/tutorials/hello-ratatui`...🔧   Initializing a fresh Git repository✨   Done! New project created /Users/joshka/local/ratatui-website/code/tutorials/hello-ratatui
```

## # Examine the Project

[Section titled “Examine the Project”](#examine-the-project)

The `cargo generate` command creates a new folder called `hello-ratatui` with a basic binary
application in it. If you examine the folders and files created this will look like:

```
hello-ratatui/├── src/│  └── main.rs├── Cargo.toml├── LICENSE└── README.md
```

The `Cargo.toml` file is filled with some default values and the necessary dependencies (Ratatui and
Crossterm), and one useful dependency (Color-eyre) for nicer error handling.

```
[package]name = "hello-ratatui"version = "0.1.0"authors = ["Josh McKinney "]license = "MIT"edition = "2021"
[dependencies]color-eyre = "0.6.3"crossterm = "0.28.1"ratatui = "0.29.0"
```

"]license = "MIT"edition = "2021"[dependencies]color-eyre = "0.6.3"crossterm = "0.28.1"ratatui = "0.29.0"" data-copied="Copied!" title="Copy to clipboard"\>

The generate command created a default `main.rs` that runs the app:

```
use color_eyre::Result;use crossterm::event::{self, Event};use ratatui::{DefaultTerminal, Frame};
fn main() -> Result<()> {    color_eyre::install()?;    let terminal = ratatui::init();    let result = run(terminal);    ratatui::restore();    result}
fn run(mut terminal: DefaultTerminal) -> Result<()> {    loop {        terminal.draw(render)?;        if matches!(event::read()?, Event::Key(_)) {            break Ok(());        }    }}
fn render(frame: &mut Frame) {    frame.render_widget("hello world", frame.area());}
```

## # Run the App

[Section titled “Run the App”](#run-the-app)

Let’s build and execute the project. Run:

```
cd hello-ratatuicargo run
```

You should see the build output and then a TUI app with a `Hello world` message.

<img alt="hello" decoding="async" fetchpriority="auto" height="450" loading="lazy" src="/_astro/hello-ratatui.BUSC3RMX_Z16g3cK.webp" width="1200">

You can press any key to exit and go back to your terminal as it was before.

Summary
- ---------

[Section titled “Summary”](#summary)

Congratulations! 🎉 You have written a “hello world” terminal user interface with Ratatui. The
next sections will go into more detail about how Ratatui works.

The next tutorial, [Counter App](/tutorials/counter-app/), introduces some more interactivity, and a
more robust approach to arranging your application code.