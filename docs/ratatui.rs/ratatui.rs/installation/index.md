# Installation

`ratatui` is a standard rust crate and can be installed into your app using the following command:

```
cargo add ratatui
```

or by adding the following to your `Cargo.toml` file:

```
[dependencies]ratatui = "0.28.0"
```

By default, `ratatui` enables the `crossterm` feature, but it’s possible to alternatively use`termion`, or `termwiz` instead by enabling the appropriate feature and disabling the default
features. See [Backend](/concepts/backends/) for more information.

For Termion:

```
cargo add ratatui --no-default-features --features termion
```

or in your `Cargo.toml`:

```
[dependencies]ratatui = { version = "0.28.0", default-features = false, features = ["termion"] }
```

For Termwiz:

```
cargo add ratatui --no-default-features --features termwiz
```

or in your `Cargo.toml`:

```
[dependencies]ratatui = { version = "0.28.0", default-features = false, features = ["termwiz"] }
```