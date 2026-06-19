# bevy_dioxus_sync

A crate for bevy dioxus integration.

[bevy_dioxus_sync_V1.webm](https://github.com/user-attachments/assets/7ab6ca08-9d26-4323-b19c-93bf57d8485c)

## Features

- Signals-based hooks for sending resources, components, and assets between dioxus and bevy.
- Synchronization for window resize, keyboard input, and mouse events between bevy and dioxus.
- Native rendering through blitz

## To use

See the demos in `/examples/minimal` for a working setup.

## To run

To serve bevy_dioxus_sync apps with dioxus hot-patching, ensure your dioxus-cli version matches bevy_dioxus_sync's version of dioxus.

```cli
cargo install dioxus-cli@<current-dioxus-version> --locked
```

Then run:

```cli
dx serve --package minimal --hot-patch
```
