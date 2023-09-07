#!/usr/bin/env bash

set -e

pushd $(git rev-parse --show-toplevel)
  fly deploy -c caje.fly.toml --dockerfile caje.Dockerfile
popd
