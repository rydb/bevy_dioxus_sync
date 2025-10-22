# bevy_dioxus_sync

bevy-dioxus interop between Dioxus and bevy to syncronize their state

!add picture here when <https://github.com/DioxusLabs/dioxus/issues/4616> gets fixed.

## Features

- hooks for sending resources, components, and assets between dioxus and bevy.
- syncronization for (some) events to and from dioxus (window resize, keyboard input).
- native support through the ✨ new ✨ `dioxus-native` renderer.

## To use

See hooks/{hook}.rs hook files for available hooks, or see demos in `/examples`.

## To run

to serve bevy_dioxus_sync apps with dioxus hot-patching

ensure your dioxus-cli version matches bevy_dioxus_sync's version of dioxus.

```cli
cargo install dioxus-cli@<current-dioxus-version> --locked
```

then run:

```cli
dx serve (--package OR --example) <your project> --hotpatch
```

e.g: for `minimal`

```cli
dx serve --package minimal --hot-patch
```
