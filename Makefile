# HULKForge build — satisfies the Matcom Compilers interface contract.
# `make build` compiles the release binary and exposes it as ./hulk at the repo root.
# `build` is the first/default target, so plain `make` also builds.

build:
	cargo build --release
	cp target/release/hulk_forge ./hulk

clean:
	cargo clean
	rm -f ./hulk ./output

.PHONY: build clean
