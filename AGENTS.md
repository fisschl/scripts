# AGENTS.md

This file contains guidelines for agentic coding assistants working on this Rust codebase.

## Build and Test Commands

```bash
# Run with specific subcommand
cargo run -- hash-copy --source ./photos --target ./backup
cargo run -- compress-delete --directory ./projects

# Run tests
cargo test

# Run a specific test (if tests exist)
cargo test test_name

# Check code without building
cargo check

# Format code
cargo fmt

# Lint code (clippy)
cargo clippy
```

## Code Style Guidelines

### Documentation

- All public functions must have doc comments with `///`
- Module-level docs use `//!` at file top
- Document behavior, not just implementation

### Code Organization

- Commands in `src/commands/` (one file per command)
- Utilities in `src/utils/` (one file per utility category)
- Export commands in `src/commands.rs` and utils in `src/utils.rs`
- Use `pub` for public functions/types

### Utils 模块使用

优先使用已有的 utils 函数。

- `compress.rs`: `find_7z()`, `compress_7z()`
- `filesystem.rs`: `get_file_extension()`, `calculate_dir_size()`
- `hash.rs`: `calculate_file_hash()`
- `media.rs`: `test_encoder()`, `detect_av1_encoder()`, `transcode_to_webm_av1()`, `transcode_to_mp4_av1()`

### Additional Notes

- This is a Windows-focused CLI tool (7-Zip, Windows paths)
- Uses Chinese comments and documentation in the codebase
- Deletion operations use `trash::delete()` to move files to recycle bin
- Prioritize user safety - confirm before destructive operations
- All external commands (7z, ffmpeg) should inherit stdout/stderr
