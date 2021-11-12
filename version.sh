#!/usr/bin/env bash

echo $(awk -F'[ ="]+' '$1 == "version" { print $2 }' Cargo.toml)
