_:
  @just -l

build:
  cargo xtask bundle one-dot-909 --release  # -F nih_plug/standalone

install: build
  cp -rf ./target/bundled/one-dot-909.vst3 ~/.vst3
  cp -rf ./target/bundled/one-dot-909.clap ~/.clap
  # cp -rf ./target/bundled/one-dot-909 ~/.local/bin/one-dot-909

