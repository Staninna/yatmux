# yatmux

A small terminal emulator with tabs, splits, and a themeable UI.

## Documentation

- Start here: `docs/README.md`

## Quick run (from source)

- `cargo run -r`
- Or use the Makefile: `make run`

On first launch, yatmux writes a commented config template (when a config directory can be determined). Reload config at runtime with `ctrl+shift+r`.

## Development

- **Development Setup**: `scripts/dev/setup.sh`
- **Quick Checks**: `make quick-check` or `scripts/dev/quick-check.sh`
- **Development Mode**: `make run` or `scripts/dev/run-dev.sh`
- **Watch Mode**: `make watch` or `scripts/dev/watch.sh`
- **Documentation**: See `AGENTS.md` for LLM development guidelines

See the [Makefile](./Makefile) for all available commands.
