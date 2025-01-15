#!/bin/env sh

mkdir -p target/web
cp static/* target/web
rustup run nightly wasm-pack build --debug --weak-refs --reference-types --target web --no-typescript --no-pack -d target/web/pkg
rm target/web/pkg/.gitignore
tar -C target -zcf target/web.tar.gz web
echo "Package created: target/web.tar.gz"
