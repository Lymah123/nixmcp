# NixMCP — Initial Architecture RFC

**Status:** Draft  
**Version:** 0.1  
**Date:** 2026-08-13

## 1. Summary

NixMCP is an MCP server that gives AI assistants direct access to live Nix ecosystem data.

The goal is to allow MCP-compatible clients such as ChatGPT, Claude, Cursor, Zed, and other assistants to query Nix packages, NixOS options, and local Flakes without relying on model memory.

The core principle is:

> Prefer live, authoritative Nix data over model-generated guesses.

NixMCP is initially designed as a local MCP server communicating with clients over stdio.

---

## 2. Problem

AI assistants frequently hallucinate when answering questions about rapidly changing technical ecosystems.

Nix is particularly susceptible because:

- package versions change frequently;
- package attributes can differ between nixpkgs revisions;
- NixOS options evolve;
- Flake inputs and outputs are project-specific;
- package metadata is revision-dependent.

An AI model should not be expected to memorize this information.

NixMCP allows the assistant to query the actual Nix environment instead.

---

## 3. Goals

### MVP goals

The first release will provide five tools:

1. `search_packages`
2. `get_package`
3. `search_options`
4. `get_option`
5. `inspect_flake`

The MVP should:

- communicate through MCP;
- use the official Rust MCP SDK;
- query authoritative Nix data;
- return deterministic structured responses;
- provide useful errors;
- work without requiring a database;
- remain small enough to understand and contribute to.

### Non-goals for the MVP

The MVP will not initially include:

- dependency graph analysis;
- build failure analysis;
- security/CVE intelligence;
- Home Manager-specific tools;
- changelog generation;
- persistent databases;
- remote indexing infrastructure;
- plugin systems.

These may be introduced in later phases.

---

## 4. MCP SDK

NixMCP will use the official Rust MCP SDK:

`rmcp`

The project will use the current compatible 3.x release line.

MCP tool definitions will use the SDK's macro-based tool routing where appropriate.

The initial transport will be stdio because it is widely supported by local MCP clients and keeps the MVP simple.

---

## 5. Architecture

The initial application will be a single Rust crate.

The codebase will use modules rather than multiple Cargo crates.

```text
src/
├── cache/
├── clients/
│   └── nix.rs
├── models/
├── server/
├── tools/
│   ├── search_packages.rs
│   ├── get_package.rs
│   ├── search_options.rs
│   ├── get_option.rs
│   └── inspect_flake.rs
├── error.rs
└── main.rs
```

## 6. Data Sources

**Nix CLI**

The primary source for package and Flake information will be the local Nix installation.

Relevant commands may include:

- `nix search`
- `nix eval`
- `nix flake show`

Nix commands will be executed as subprocesses rather than reimplementing Nix evaluation logic inside NixMCP.

This keeps NixMCP close to the behavior of the user's installed Nix environment.

### NixOS options

The initial options implementation will use an authoritative generated NixOS options dataset.

The exact channel/source strategy will be finalized before implementing the options tools.

The implementation must clearly identify the channel or revision represented by the data.

### External APIs

GitHub, Hydra, and other external services are outside the MVP unless required by an MVP tool.

## 7. Nix Compatibility

The initial development environment uses:

- Nix 2.33.x
- `nix-command`
- `flakes`

NixMCP must detect whether the `nix` executable is available.

If Nix is unavailable, tools requiring the local Nix CLI should return a clear structured error rather than silently producing fabricated results.

The minimum supported Nix version will be finalized before the first release.

Experimental feature requirements will be documented rather than assumed.

## 8. MVP Tools

`search_packages`

Search for packages matching a query.

Example:

```
search_packages("ripgrep")
```

Expected information includes:

- package name;
- description;
- version;
- license;
- platforms.

`get_package`

Retrieve information about a specific package.

Example:

```
get_package("ripgrep")
```

Expected information includes:

- version;
- homepage;
- maintainers;
- dependencies;
- source location.

`search_options`

Search NixOS options.

Example:

```
search_options("openssh")
```

Expected results may include:

- services.openssh.enable
- services.openssh.settings.PasswordAuthentication
- services.openssh.ports

`get_option`

Retrieve detailed information about a NixOS option.

Example:

```
get_option("services.openssh.enable")
```

Expected information includes:

- description;
- type;
- default;
- example.

`inspect_flake`

Inspect a local Flake.

Example:

```
inspect_flake("./")
```

Expected information includes:

- inputs;
- outputs;
- systems.

## 9. Response Design

Tool responses should be:

- deterministic;
- structured;
- concise;
- useful to an AI model;
- explicit about data provenance where relevant.

Empty searches should return a successful response containing an empty result set rather than treating "no matches" as an internal failure.

Operational failures such as:

- missing Nix executable;
- invalid Flake;
- failed Nix command;
- malformed data;

should return explicit errors.

NixMCP must never fabricate package or option information when authoritative data cannot be obtained.

## 10. Caching

The MVP will not require a database.

Caching may be introduced as a lightweight module to reduce repeated expensive operations.

The cache must not undermine the project's live-data principle.

Cached responses should have a defined invalidation or freshness strategy.

## 11. Testing

The project will maintain three basic quality gates:

```
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

CI will run these checks on pull requests and pushes to **main**.

Tool implementations should eventually include unit and integration tests around:

- command execution;
- parsing;
- error handling;
- response schemas;
- representative Nix data.

## 12. Development Workflow

Development will use short-lived feature branches.

Example:

```
main
  |
  └── feature/package-search
          |
          └── Pull Request
                    |
                    └── main
```
Each feature should:

- have a focused scope;
- pass formatting;
- pass Clippy;
- pass tests;
- be reviewed before merging.

**main** should remain in a working state.

## 13. Future Phases

**Phase 2**

Potential features:

- dependency graphs;
- reverse dependencies;
- build-log analysis;
- version comparison;
- changelog analysis;
- security information;
- Home Manager options.

**Phase 3**

Potential infrastructure:

- persistent/incremental indexing;
- advanced caching;
- streaming;
- plugin architecture;
- broader Nix ecosystem integrations.

These features are intentionally outside the MVP.

## 14. Open Questions

The following decisions remain open until implementation research provides enough evidence:

1. Exact Nix minimum supported version.
2. Exact NixOS options data source and update strategy.
3. Whether package metadata should always come directly from the local Nix environment or support additional remote sources.
4. Cache format and invalidation strategy.
5. Exact MCP response schemas for each tool.
6. Whether future remote deployments should use an additional MCP transport.

These decisions should be resolved before they become implementation constraints.

## 15. Design Principles

NixMCP follows several principles:

**Live data over model memory**

If Nix can answer the question, query Nix.

**No hallucinated infrastructure**

When authoritative data is unavailable, return an explicit error.

**Small composable tools**

Each MCP tool should have one clear responsibility.

**Deterministic behavior**

The same Nix environment and request should produce predictable results.

**Simple first**

Avoid introducing infrastructure before the problem requires it.

**Observable source of truth**

Where practical, responses should make their data source or Nix revision/channel clear.

**Build in public**

The project should document architectural decisions, implementation lessons, and discoveries as development progresses.