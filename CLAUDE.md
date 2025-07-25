# CLAUDE.md - Development Guidelines

## Coding Standards

### Code Quality
- Always run `cargo clippy -- -D warnings` before committing
- Use modern format args: `println!("Value: {value}")` not `println!("Value: {}", value)")`
- Remove unused imports and dead code
- Prefer simple, readable code over complex abstractions

### Git Requirements
- **CRITICAL: Always commit changes to git**
- All changes must be committed with clear, concise messages
- Commit regularly during development

### Architecture Patterns
- Simple struct + function organization
- Direct database connections with proper error handling
- Single responsibility functions
- Minimal abstractions unless solving real problems

### Anti-Patterns to Avoid
- Builder patterns for simple structs
- Repository patterns with connection pooling for single connections
- Phantom types without clear benefit
- Verbose academic documentation

## Known Issues

### Missing Behavior Tracking (CRITICAL)
The muse recommendation engine needs behavior tracking for learning:
- Functions exist: `update_song_stats()`, `handle_song_finished()`, `handle_song_skipped()`
- Problem: Not called anywhere, marked with `#[allow(dead_code)]`
- Impact: Algorithm never learns from user behavior
- Solution needed: Implement MPD event monitoring or command wrappers

### Available Commands
Commands implemented: `init-db`, `update`, `list`, `play`, `current`, `thread`, `stream`, `completion`, `completion-enhanced`, `next`, `skip`, `love`, `unlove`, `info`, `daemon`

## Development Process
1. Run `cargo clippy -- -D warnings`
2. Make changes
3. Test functionality
4. Commit to git with descriptive message
5. Repeat

**Priority: Fix behavior tracking to enable algorithm learning**
