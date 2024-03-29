.PHONY: cli wrapper daemon runner
.DEFAULT_GOAL := all

download-wasi-sdk:
	./install_wasi_sdk.sh

all: wrapper cli runner daemon mediator cli-tools

cli: wrapper
		cd crates/cli && cargo build --release && cd -

wrapper:
	cd crates/wrapper \
		&& QUICKJS_WASM_SYS_WASI_SDK_PATH="$(shell pwd)/crates/wrapper/wasi-sdk" cargo build --release --target=wasm32-wasi \
		&& cd -

runner:
	cd crates/runner && cargo build --release && cd -

daemon:
	cd crates/daemon \
		&& cargo build --release \
		&& cd -

mediator:
	cd mediator; \
		mvn package \
		&& cd -

cli-tools:
	cd cli && \
		mvn package \
		&& cd -

test-runner:
		cd crates/runner \
				&& cargo test -- --nocapture \
				&& cd -

docker-native:
	docker build . -f ./Dockerfile.native

docker-managed:
	docker build . -f ./Dockerfile.managed

docker: docker-native docker-managed

tests: test-runner

fmt: fmt-cli fmt-daemon fmt-runner fmt-wrapper

fmt-cli:
		cd crates/cli/ \
				&& cargo fmt -- \
				&& cargo clippy -- -D warnings \
				&& cd -

fmt-daemon:
		cd crates/daemon/ \
				&& cargo fmt -- \
				&& cargo clippy -- -D warnings \
				&& cd -

fmt-runner:
		cd crates/runner/ \
				&& cargo fmt -- \
				&& cargo clippy -- -D warnings \
				&& cd -

fmt-wrapper:
		cd crates/core/ \
				&& cargo fmt -- \
				&& cargo clippy --target=wasm32-wasi -- -D warnings \
				&& cd -

fmt-mediator:
		cd crates/mediator/ \
				&& cargo fmt -- \
				&& cargo clippy --target=wasm32-wasi -- -D warnings \
				&& cd -

clean: clean-wasi-sdk clean-cargo clean-mediator clean-cli-tools

clean-cargo:
		cargo clean

clean-wasi-sdk:
		rm -r crates/wrapper/wasi-sdk 2> /dev/null || true

clean-mediator:
	cd mediator && mvn clean

clean-cli-tools:
	cd cli && mvn clean