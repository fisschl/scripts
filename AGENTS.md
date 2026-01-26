# AGENTS.md

This file contains guidelines for agentic coding assistants working on this Rust codebase.

## Code Style Guidelines

### Documentation
- All public functions must have doc comments with `///`
- Module-level docs use `//!` at file top
- Document behavior, not just implementation

### Code Organization
- Commands in `src/commands/` (one file per command)
- Utilities in `src/utils/` (one file per utility category)
- Export commands in `src/commands.rs` and utils in `src/utils.rs`

### Utils Module
Prioritize using existing utils functions, do not reimplement.
- `compress.rs`: `find_7z()`, `compress_7z()`
- `filesystem.rs`: `get_file_extension()`, `calculate_dir_size()`
- `hash.rs`: `calculate_file_hash()`
- `media.rs`: `test_encoder()`, `detect_av1_encoder()`, `transcode_to_webm_av1()`, `transcode_to_mp4_av1()`

### Additional Notes
- Windows-focused CLI tool (7-Zip, Windows paths)
- Uses Chinese comments and documentation
- Deletion uses `trash::delete()` to move to recycle bin
- External commands (7z, ffmpeg) inherit stdout/stderr

## Post-Coding Workflow

After completing code changes, run:
```bash
cargo check
cargo fmt
cargo clippy
```
