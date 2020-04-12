#!/bin/bash
docker run -ti --volume $PWD:/home/cross/project  --volume ~/.cargo/registry:/home/cross/.cargo/registry ragnaroek/rust-raspberry:1.42.0 build
scp target/arm-unknown-linux-gnueabihf/debug/garden-buttler pi@raspberrypi:~/garden-buttler
scp layout.json pi@raspberrypi:~/layout.json
scp watering-schedules.json pi@raspberrypi:~/watering-schedules.json
