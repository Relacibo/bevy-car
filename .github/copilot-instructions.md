# GitHub Copilot Instructions

## Project Context
This is a Bevy game engine project using Rust.

**Rust Edition**: 2024

## Bevy Version
- **Current version: Bevy 0.18.1**
- Many AI models are trained on older Bevy versions (0.14 or earlier)
- **CRITICAL**: Read and follow `.github/bevy.md` for current API patterns

## Important Guidelines

### 1. Bevy API Usage
**ALWAYS consult `.github/bevy.md` before suggesting Bevy code.**

Key points to remember:
- Use `Single` system parameter instead of `Query::single()`
- NO `MaterialMesh2dBundle` - use `Mesh2d` + `MeshMaterial2d` components
- NO `TextBundle` - use `Text`, `TextFont`, `TextColor` components
- **Prefer** `children![]` macro for hierarchies (use `with_children` only for dynamic logic)
- Query methods return `Result` - handle errors properly
- Use new event system: `MessageReader`/`MessageWriter`

### 2. Code Quality
- Write idiomatic Rust code
- Prefer explicit types over magic bundles
- Handle errors with `Result` and `Option`
- Keep systems focused and composable

### 3. When Suggesting Code
1. First check if the pattern is in `.github/bevy.md`
2. Use current Bevy 0.18.x APIs only
3. Avoid deprecated functions and bundles
4. Explain any Bevy-specific patterns if asked

### 4. Common Mistakes to Avoid
- ❌ Using `Query::single()` without `Result`
- ❌ Suggesting `MaterialMesh2dBundle` or similar deprecated bundles
- ❌ Suggesting `TextBundle` - use explicit `Text` components
- ❌ Using old weak handle APIs
- ❌ Panicking queries for unique entities
- ❌ Manual `push_children` instead of `children![]` macro

---

**For detailed Bevy 0.18.x API guidance, see: `.github/bevy.md`**
