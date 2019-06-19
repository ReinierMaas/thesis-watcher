# thesis-watcher
Watches my thesis and calls `make` when files are changed.

Watching from a different current directory use: `cargo run --release --manifest-path <path/to/Cargo.toml> -- <extensions: tex bib>`.

Watching a different directory use: `cargo run --release -- -w <path/to/different/directory> <extensions: tex bib>`.
