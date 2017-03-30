EMSCRIPTEN_ENV = ../../../emscrip/emsdk_portable

MAIN = mass_nes

export PATH := $(PATH):$(EMSCRIPTEN_ENV):$(EMSCRIPTEN_ENV)/clang/fastcomp/build_incoming_64/bin:$(EMSCRIPTEN_ENV)/node/4.1.1_64bit/bin:$(EMSCRIPTEN_ENV)/emscripten/incoming

export EM_CONFIG = /home/nickmass/.emscripten
export EMSCRIPTEN = $(EMSCRIPTEN_ENV)/emscripten/incoming

all: $(MAIN)
	rustc -O --target=wasm32-unknown-emscripten src/main.rs --extern nes=target/release/deps/libnes.rlib --out-dir target/release
	cp src/index.html target/release

mass_nes:
	mkdir -p target/release/deps
	rustc -O --target=wasm32-unknown-emscripten ../nes/lib.rs --crate-name nes --crate-type lib -g --out-dir target/release/deps --emit=dep-info,link
