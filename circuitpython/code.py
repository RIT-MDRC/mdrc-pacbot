import time
import board
import digitalio
import adafruit_vl6180x
import busio

from pacbot import *

while True:
    sensors = []
    for i in range(8):
        sensors.append(dist_sensors[i].range)
    for i in range(8):
        print(sensors[i])
    time.sleep(.05)
    break

print("Ok!")
