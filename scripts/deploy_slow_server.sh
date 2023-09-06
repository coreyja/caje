#!/usr/bin/env bash

set -e

pushd $(git rev-parse --show-toplevel)
  fly deploy -c slow_server.fly.toml --dockerfile slow_server.Dockerfile
popd
