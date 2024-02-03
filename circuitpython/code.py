# code.py with PID

import wifi
import socketpool
import ipaddress
import time
import board
import digitalio
import struct

from pacbot import *

class PID:
    def __init__(self, kp, ki, kd):
        self.kp = kp
        self.ki = ki
        self.kd = kd
        self.previous_error = 0
        self.integral = 0

    def compute(self, setpoint, measured_value):
        error = setpoint - measured_value
        self.integral += error
        derivative = error - self.previous_error
        output = self.kp * error + self.ki * self.integral + self.kd * derivative
        self.previous_error = error
        return output

# Initialize PID controllers for each motor
pid_a = PID(3000.0, 100.0, 0.1)  # Tune these values
pid_b = PID(3000.0, 100.0, 0.1)  # Tune these values
pid_c = PID(3000.0, 100.0, 0.1)  # Tune these values

# Previous encoder positions and time
previous_positions = [0, 0, 0]
previous_time = time.monotonic()

print("Connecting to wifi")
wifi.radio.connect("testnetwork", "password123")
pool = socketpool.SocketPool(wifi.radio)

print("Self IP", wifi.radio.ipv4_address)

print("Create UDP Client socket")
s = pool.socket(pool.AF_INET, pool.SOCK_DGRAM)
s.bind(("0.0.0.0", 20001))

buf = bytearray(7)
old_motors = [1, 0, 0, 0, 1, 1, 0]

desired_speeds = [0, 0, 0]

print("Receiving UDP messages")
while True:
    current_time = time.monotonic()
    time_interval = current_time - previous_time
    previous_time = current_time

    size, addr = s.recvfrom_into(buf)

    if size == 7:
        try:
            # send data
            format_str = 'B' * 8 + 'i' * 3
            dist_sensors = [13, 255, 13, 0, 0, 13, 13, 255]
            data = tuple(dist_sensors) + (encoders[0].position, encoders[1].position, encoders[2].position)
            packed_data = struct.pack(format_str, *data)
            s.sendto(packed_data, addr)
        except:
            print("Failed to send message")
        if list(buf) != old_motors:
            old_motors = list(buf)
            print("New motors: " + str(old_motors))

            desired_speeds = [(old_motors[1] / 255.0) * 11.6, (old_motors[2] / 255.0) * 11.6, (old_motors[3] / 255.0) * 11.6]
            if desired_speeds[0] == 0:
                pid_a = PID(3000.0, 100.0, 0.1)
            if desired_speeds[1] == 0:
                pid_b = PID(3000.0, 100.0, 0.1)
            if desired_speeds[2] == 0:
                pid_c = PID(3000.0, 100.0, 0.1)
    actual_speeds = []
    for i in range(3):
        speed = (encoders[i].position - previous_positions[i]) / time_interval / 150
        actual_speeds.append(abs(speed))
        previous_positions[i] = encoders[i].position

    for i in range(3):
        if MOTORS_ENABLED[i]:
            forward = old_motors[i + 4] == 2
            pid_output = [pid_a, pid_b, pid_c][i].compute(desired_speeds[i], actual_speeds[i])
            turn_motor(motor_pwm_pins[i], speed=max(min(65534, int(pid_output)), 0), forward=forward)
