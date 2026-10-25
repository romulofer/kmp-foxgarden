# kmp-foxgarden

Kotlin Multiplatform support for [FoxGarden](../foxgarden), as an extension
rather than as part of the editor.

It reaches the editor only through `fg-extension`, like
[`spring-foxgarden`](../spring-foxgarden), and builds on that extension for
everything Kotlin itself. Both checkouts must sit beside `foxgarden`.

## What it contributes today

- **Source sets**: for a file in a module applying the Gradle `multiplatform`
  plugin, the source set it belongs to (`commonMain`, `jvmTest`, ...), shown in
  the status bar. Read from the path and the module's build file; no Gradle
  run.
- **`expect`/`actual` navigation**: from the declaration on a line, the
  matching `actual`s of an `expect` (or the `expect` of an `actual`) across the
  module's source sets, matched by name. Offered by the command palette's
  "Go to expect/actual".

- **Missing `actual`**: a warning on an `expect` that no `actual` of the same
  name in the module fulfils, refreshed when a file opens, reloads or is saved.

Planned: per-target `actual` checks, per-target build/run/test tasks, then per-source-set language server
classpaths.

## Build

```bash
cargo test --workspace
```

## License

MIT, see [LICENSE](LICENSE).
