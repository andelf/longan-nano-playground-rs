#!/bin/sh

set -e

cargo build --examples --release --all-features
