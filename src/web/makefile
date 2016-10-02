EMSCRIPTEN_RUSTC = ../../../emscrip/rust/build/x86_64-unknown-linux-gnu/stage2/bin/rustc
EMSCRIPTEN_ENV = ../../../emscrip/emsdk_portable

MAIN = mass_nes

export PATH := $(PATH):$(EMSCRIPTEN_ENV):$(EMSCRIPTEN_ENV)/clang/fastcomp/build_incoming_64/bin:$(EMSCRIPTEN_ENV)/node/4.1.1_64bit/bin:$(EMSCRIPTEN_ENV)/emscripten/incoming

export EM_CONFIG = /home/nickmass/.emscripten
export EMSCRIPTEN = $(EMSCRIPTEN_ENV)/emscripten/incoming

all: $(MAIN)
	$(EMSCRIPTEN_RUSTC) -O --target=asmjs-unknown-emscripten src/main.rs --extern mass_nes=target/debug/deps/libmass_nes.rlib --out-dir target/debug
	cp src/index.html target/debug
	cp src/worker.js target/debug

mass_nes:
	mkdir -p target/debug/deps
	$(EMSCRIPTEN_RUSTC) -O --target=asmjs-unknown-emscripten ../lib.rs --crate-name mass_nes --crate-type lib -g --out-dir target/debug/deps --emit=dep-info,link
