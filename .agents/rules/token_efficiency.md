# Token Efficiency & Lazy Context Rules

Always follow these rules to minimize token usage and latency during code analysis, editing, and planning:

## 1. Lazy Loading & Minimal Inspection
- **Search Before Reading**: Use `grep_search` to locate exact symbols, functions, or variable names instead of reading entire files.
- **Slice Views**: When inspecting code, use `view_file` with precise `StartLine` and `EndLine` parameters rather than viewing full 500+ line files.
- **Do Not Re-quote Code**: When explaining changes, highlight only modified symbols or diff snippets rather than echoing large blocks of unchanged code.

## 2. Minimal File Edits
- **Use Precise Diffs**: Always use `replace_file_content` with concise target/replacement chunks rather than rewriting entire files.
- **Do Not Reformat Unrelated Code**: Keep diffs tight to prevent unnecessary token generation.

## 3. Ignore Non-Source Artifacts
- Never read or ingest files excluded by `.geminiignore` (e.g. `*.svg`, `Cargo.lock`, `package-lock.json`, binary files, database WAL journals, or build outputs).
