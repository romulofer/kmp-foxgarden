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

Planned: `expect`/`actual` navigation and a missing-`actual` diagnostic,
per-target build/run/test tasks, then per-source-set language server
classpaths.

## Build

```bash
cargo test --workspace
```

## License

MIT, see [LICENSE](LICENSE).
