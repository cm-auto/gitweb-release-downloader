#!/usr/bin/env bash

cargo run -- download -w gitlab gitlab.com/fdroid/fdroidclient "\.apk$" "$@" -i ipv4