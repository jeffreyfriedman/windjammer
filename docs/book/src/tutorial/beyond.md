# Beyond the language

You now have the whole language. Real projects also need libraries, a way to put something on screen, and examples to
copy. Two public repositories alongside the compiler cover that.

## The ecosystem

[windjammer-ecosystem](https://github.com/jeffreyfriedman/windjammer-ecosystem) is a collection of small packages and
demo apps written in Windjammer. It shows what everyday development looks like: command-line tools, small APIs, config
libraries, and the building blocks they share.

* **Wrappers over the standard library**: hashing, JWTs, regular expressions, YAML, compression, a directory walker, an
  HTTP client.
* **Parsers for common formats**: TOML, CSV, cron expressions, URLs and query strings, semantic versions, durations,
  globs, templates, dotenv files, command-line arguments, cookies.
* **HTTP building blocks**: a router, CORS, headers, rate limiting, validation.
* **Runtime pieces**: channels and pools, an event bus, retries, logging, configuration, migrations, UUIDs.

Each package is an ordinary Windjammer project, so you use one the way the [modules chapter](modules.md) describes: a path
or git entry under `[dependencies]` in your `wj.toml`, then `use package::item`. Every package declares its own
capabilities, so adding one shows up in `wj caps show` as a visible change in what your program may do; see
[Effects and capabilities](effects.md).

The demo apps are complete programs worth reading: a note-taking API, a static site generator, a webhook receiver, a file
finder, a scheduler and more. They are written in the style this book teaches.

## User interfaces

[windjammer-ui](https://github.com/jeffreyfriedman/windjammer-ui) is a component library and reactive runtime. You write
components in Windjammer as builders that describe a tree of elements; a reactive core with signals and a keyed patcher
updates the page in the browser through WebAssembly. It uses the same builder style you met in the
[ownership chapter](ownership.md), and the WebAssembly target from [Targets](targets.md).

## Real-time graphics

Programs that draw, simulate or compute on the graphics card need two things: Windjammer code on the CPU and shader code on
the GPU. Windjammer covers the second with its own shader language, WJSL, which compiles to WGSL, and with a backend that
lays out the host-side structs the shaders read. [Shaders](shaders.md) is a complete chapter on both.

## Tools around the compiler

* **Editors**: a language server and a VS Code extension ship in the compiler repository.
* **Agents**: structured diagnostics, an offline documentation search (`wj docs`) and an MCP server give coding agents
  the same loop you use.
* **`wj eject`**: writes the Rust project behind any Windjammer project, so there is always a way out.

## Where to read next

The [reference](../reference/language.md) is the place for exact rules. The [cookbook](../cookbook/recipes.md) has short
complete answers. The compiler repository's `docs` directory holds the language specification, the design decisions
behind it, and the list of known gaps.
