.PHONY: help venv develop release build test test-rust test-python fmt lint clean

VENV := .venv
PYTHON := $(VENV)/bin/python

help:
	@echo "Targets:"
	@echo "  venv        create .venv and install maturin + dev dependencies (via uv)"
	@echo "  develop     build the Rust extension and install it into .venv (debug, fast to compile)"
	@echo "  release     build the Rust extension in release mode and install it into .venv"
	@echo "              (use this to actually run the CLI on real capture files - debug"
	@echo "              builds are ~10x slower on multi-million-line input)"
	@echo "  build       build an optimized release wheel"
	@echo "  test        build the extension and run the full test suite"
	@echo "  test-rust   run Rust unit tests (cargo test)"
	@echo "  test-python run Python tests (pytest) against the installed extension"
	@echo "  fmt         format Rust and Python sources"
	@echo "  lint        run cargo clippy"
	@echo "  clean       remove build artifacts"

$(PYTHON):
	uv venv $(VENV)
	uv pip install --python $(PYTHON) maturin pytest

venv: $(PYTHON)

develop: venv
	$(VENV)/bin/maturin develop

release: venv
	$(VENV)/bin/maturin develop --release

build: venv
	$(VENV)/bin/maturin build --release

test: develop test-rust test-python

test-rust:
	cargo test

test-python:
	$(VENV)/bin/pytest

fmt:
	cargo fmt

lint:
	cargo clippy --all-targets -- -D warnings

clean:
	cargo clean
	rm -rf $(VENV) .pytest_cache src/netconfig_splitter/*.so src/netconfig_splitter/*.pyd

