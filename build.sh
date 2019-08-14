#!/bin/bash
docker run -ti --volume $PWD:/home/cross/project  --volume ~/.cargo/registry:/home/cross/.cargo/registry --volume $PWD/../tokio-chrono:/home/cross/tokio-chrono ragnaroek/rust-raspberry:1.36.1 build
scp target/arm-unknown-linux-gnueabihf/debug/garden-buttler pi@raspberrypi:~/garden-buttler
scp layout.json pi@raspberrypi:~/layout.json
scp watering-schedules.json pi@raspberrypi:~/watering-schedules.json
