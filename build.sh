#!/bin/bash
cargo build --target=armv7-unknown-linux-gnueabihf
scp target/armv7-unknown-linux-gnueabihf/debug/garden-buttler pi@raspberrypi:~/garden-buttler
