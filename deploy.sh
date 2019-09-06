#!/bin/bash
scp target/arm-unknown-linux-gnueabihf/debug/garden-buttler pi@raspberrypi:~/garden-buttler
scp layout.json pi@raspberrypi:~/layout.json
scp watering-schedules.json pi@raspberrypi:~/watering-schedules.json
scp mqtt.json pi@raspberrypi:~/mqtt.json
