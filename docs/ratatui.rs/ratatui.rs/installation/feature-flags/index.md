# Feature Flags

As ratatui grows and evolves, this list may change, so make sure to check the[main repo](https://github.com/ratatui/ratatui) if you are unsure.

Backend Selection
- ---------

[Section titled “Backend Selection”](#backend-selection)

For most cases, the default `crossterm` backend is the correct choice. See[Backends](/concepts/backends/) for more information. However, this can be changed to termion or
termwiz

```
# Defaults to crosstermcargo add ratatui
# For termion, unset the default crossterm feature and select the termion featurecargo add ratatui --no-default-features --features=termioncargo add termion
# For termwiz, unset the default crossterm feature and select the termwiz featurecargo add ratatui --no-default-features --features=termwizcargo add termwiz
```

All-Widgets
- ---------

[Section titled “All-Widgets”](#all-widgets)

This feature enables some extra widgets that are not in `default` to save on compile time. As of
v0.21, the only widget in this feature group is the `calendar` widget, which can be enabled with the`widget-calendar` feature.

```
cargo add ratatui --features all-widgets
```

Widget-Calendar
- ---------

[Section titled “Widget-Calendar”](#widget-calendar)

This feature enables the calendar widget, which requires the `time` crate.

```
cargo add ratatui --features widget-calendar
```

Serde
- ---------

[Section titled “Serde”](#serde)

Enables serialization and deserialization of style and color types using the Serde crate. This is
useful if you want to save themes to a file.

```
cargo add ratatui --features serde
```