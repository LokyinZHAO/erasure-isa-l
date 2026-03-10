## v0.3.0 (2026-03-10)

### ✨ Features

- add Galois field table support
- **erasure::ErasureCode**: add *_to_owned convenience method

### 🐛🚑️ Fixes

- **build**: broken denpendency of `erasure-isa-l-sys` in Cargo.toml; missing nasm compiler in Rust CI workflow configuration

### build

- **deps**: update rand requirement from 0.9.1 to 0.10.0

### 💚👷 CI & Build

- **github**: add dependabot and Rust CI workflow; upgrade pre-commits hooks

### 📌➕⬇️➖⬆️ Dependencies

- **rand**: bump crates rand to 0.10.0
- **erasure-isa-l-sys**: 1.0.1 -> 1.0.2

### 🚨 Linting

- fix cargo clippy lint

## v0.2.0 (2025-06-20)

### ✨ Features

- **erasure**: single source block update

## v0.1.0 (2025-06-20)

### ✨ Features

- basic bindings and high-level abstractions for isa-l

### ✅🤡🧪 Tests

- **isal**: add test

### 📝💡 Documentation

- **crate::erasure**: update docs
- add README.md and update cargo package information
