#!/usr/bin/env bash

cargo run -- download --website-type gitea codeberg.org/forgejo/forgejo ".*" "$@" -i ipv4