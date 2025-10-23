# Counter App

The previous [Hello Ratatui](/tutorials/hello-ratatui) tutorial introduced how to create a simple TUI that displayed some text
and waited for the user to press a key. This tutorial will cover how to handle state and some more
complex interactions. You will build a counter application that allows the user to increment and
decrement a value on the screen.

When you’re finished the application will look like the following:

<img alt="basic-app demo" decoding="async" fetchpriority="auto" height="400" loading="lazy" src="/_astro/basic-app.Ct70-NG2_9Kk2v.webp" width="1200">

The application will render the counter in a [`Paragraph`](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Paragraph.html) widget. When the user presses the left
and right arrow keys, the application will increment and decrement the value of the counter.