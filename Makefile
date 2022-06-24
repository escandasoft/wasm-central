.PHONY: cli wrapper daemon runner
.DEFAULT_GOAL := cli

download-wasi-sdk: ensure_wasi_sdk
	./install_wasi_sdk.sh

cli: wrapper
		cd crates/cli && cargo build --release && cd -

wrapper:

	cd crates/wrapper \
		&& QUICKJS_WASM_SYS_WASI_SDK_PATH="$(shell pwd)/crates/wrapper/wasi-sdk" cargo build --release --target=wasm32-wasi \
		&& cd -

runner:
	cd crates/runner && cargo build --release && cd -

daemon: runner
	cd crates/daemon \
		&& cargo build --release \
		&& cd -

test-runner:
		cd crates/runner \
				&& cargo test -- --nocapture \
				&& cd -

tests: test-runner

fmt: fmt-cli fmt-daemon fmt-runner fmt-wrapper

fmt-cli:
		cd crates/cli/ \
				&& cargo fmt -- --check \
				&& cargo clippy -- -D warnings \
				&& cd -

fmt-daemon:
		cd crates/daemon/ \
				&& cargo fmt -- --check \
				&& cargo clippy -- -D warnings \
				&& cd -

fmt-runner:
		cd crates/runner/ \
				&& cargo fmt -- --check \
				&& cargo clippy -- -D warnings \
				&& cd -

fmt-wrapper:
		cd crates/core/ \
				&& cargo fmt -- --check \
				&& cargo clippy --target=wasm32-wasi -- -D warnings \
				&& cd -

clean: clean-wasi-sdk clean-cargo

clean-cargo:
		cargo clean

clean-wasi-sdk:
		rm -r crates/wrapper/wasi-sdk 2> /dev/null || true
