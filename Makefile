.PHONY: help venv develop build test test-rust test-python fmt lint clean

VENV := .venv
PYTHON := $(VENV)/bin/python

help:
	@echo "Targets:"
	@echo "  venv        create .venv and install maturin + dev dependencies (via uv)"
	@echo "  develop     build the Rust extension and install it into .venv (debug)"
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

