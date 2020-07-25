#!/bin/bash
docker run -ti --volume $PWD:/home/cross/project  --volume ~/.cargo/registry:/home/cross/.cargo/registry ragnaroek/rust-raspberry:1.42.0 build
