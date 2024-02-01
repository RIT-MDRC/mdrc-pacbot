import board
import busio
import digitalio
import pwmio
import time
import rotaryio
import adafruit_vl6180x

# ----------------------- General Settings -----------------------

PWM_FREQUENCY = const(50)
MOTOR_MAX_EED = const(65535)

# Enable/Disable components

MOTORS_ENABLED = [True, True, True]
I2C_ENABLED = True
DIST_ENABLED = [True] * 8
ENCODERS_ENABLED = [True, True, True]

# ----------------------- GPIO Pin Assignment -----------------------

# always high; set to None if not needed
GPIO_ALWAYS_ON = board.GP22

# I2C bus
GPIO_I2C_SCL = board.GP16
GPIO_I2C_SDA = board.GP17

# IN/IN control for the first motor (both PWM)
# https://www.ti.com/lit/ds/symlink/drv8835.pdf#%5B%7B%22num%22%3A258%2C%22gen%22%3A0%7D%2C%7B%22name%22%3A%22XYZ%22%7D%2C0%2C720%2C0%5D
GPIO_MOTOR_A_IN_1 = board.GP20
GPIO_MOTOR_A_IN_2 = board.GP21

# IN/IN control for the second motor (both PWM)
GPIO_MOTOR_B_IN_1 = board.GP18
GPIO_MOTOR_B_IN_2 = board.GP19

# IN/IN control for the third motor (both PWM)
GPIO_MOTOR_C_IN_1 = board.GP27
GPIO_MOTOR_C_IN_2 = board.GP26

# XSHUT pins for each distance sensor - when high, distance sensor is disabled
GPIO_DIST_XSHUT = [
    board.GP8,
    board.GP4,
    board.GP3,
    board.GP2,
    board.GP5,
    board.GP6,
    board.GP7,
    board.GP9,
]

GPIO_ENCODERS = [
    (board.GP14, board.GP15),
    (board.GP12, board.GP13),
    (board.GP10, board.GP11)
]

# ----------------------- I2C Addresses -----------------------

I2C_ACCELEROMETER = const(0x68)

I2C_DIST_DEFAULT = const(0x29)
I2C_DIST_ADDRESSES = [
    0x30, # 0
    0x31,
    0x32, # 2
    0x33,
    0x34, # 4
    0x35,
    0x36, # 6
    0x37,
]

# ----------------------- Pin Setup -----------------------

if GPIO_ALWAYS_ON is not None:
    mode_pin = digitalio.DigitalInOut(GPIO_ALWAYS_ON)
    mode_pin.direction = digitalio.Direction.OUTPUT
    mode_pin.value = True

time.sleep(3)

if I2C_ENABLED:
    i2c = busio.I2C(board.GP17, board.GP16)

motor_pwm_pins = [
    (pwmio.PWMOut(GPIO_MOTOR_A_IN_1, frequency=PWM_FREQUENCY), pwmio.PWMOut(GPIO_MOTOR_A_IN_2, frequency=PWM_FREQUENCY)) if MOTORS_ENABLED[0] else (None, None),
    (pwmio.PWMOut(GPIO_MOTOR_B_IN_1, frequency=PWM_FREQUENCY), pwmio.PWMOut(GPIO_MOTOR_B_IN_2, frequency=PWM_FREQUENCY)) if MOTORS_ENABLED[1] else (None, None),
    (pwmio.PWMOut(GPIO_MOTOR_C_IN_1, frequency=PWM_FREQUENCY), pwmio.PWMOut(GPIO_MOTOR_C_IN_2, frequency=PWM_FREQUENCY)) if MOTORS_ENABLED[2] else (None, None),
]

i2c_xshut_pins = []
for dist in range(8):
    if DIST_ENABLED[dist]:
        dist_pin = digitalio.DigitalInOut(GPIO_DIST_XSHUT[dist])
        dist_pin.direction = digitalio.Direction.OUTPUT
        dist_pin.value = False
        i2c_xshut_pins.append(dist_pin)
    else:
        i2c_xshut_pins.append(())

encoders = []
for enc in range(3):
    if ENCODERS_ENABLED[enc]:
        encoders.append(rotaryio.IncrementalEncoder(GPIO_ENCODERS[enc][0], GPIO_ENCODERS[enc][1]))
    else:
        encoders.append(())

# ----------------------- Control Functions -----------------------

# IN/IN control
# pins should be a 2-tuple of PWM pins
# 0 <= speed <= 65535
# https://www.ti.com/lit/ds/symlink/drv8835.pdf#%5B%7B%22num%22%3A258%2C%22gen%22%3A0%7D%2C%7B%22name%22%3A%22XYZ%22%7D%2C0%2C720%2C0%5D
def turn_motor(pins, speed=0, forward=True, coast=False):
    if coast:
        pins[0].duty_cycle = 0
        pins[1].duty_cycle = 0
    elif speed == 0:
        pins[0].duty_cycle = 65535
        pins[1].duty_cycle = 65535
    elif forward:
        pins[0].duty_cycle = speed
        pins[1].duty_cycle = 0
    else:
        pins[0].duty_cycle = 0
        pins[1].duty_cycle = speed

def simple_turn_all_motors(speed=0, forward=True, coast=False):
    for motor in range(3):
        if MOTORS_ENABLED[motor]:
            turn_motor(motor_pwm_pins[motor], speed=speed, forward=forward, coast=coast)

dist_sensors = []
super_i2c_slave__device_address = const(0x212)
def i2c_dist_address_setup():
    print("Setting up I2C addresses..")
    for dist in range(8):
        if DIST_ENABLED[dist]:
            i2c_xshut_pins[dist].value = False
    time.sleep(0.3)
    for dist in range(0, 8):
        time.sleep(0.01)
        if DIST_ENABLED[dist]:
            print(dist)
            i2c_xshut_pins[dist].value = True
            time.sleep(0.01)
            sensor = adafruit_vl6180x.VL6180X(i2c, address=I2C_DIST_DEFAULT)
            sensor._write_8(super_i2c_slave__device_address, I2C_DIST_ADDRESSES[dist])
    for dist in range(8):
        if DIST_ENABLED[dist]:
            dist_sensors.append(adafruit_vl6180x.VL6180X(i2c, address=I2C_DIST_ADDRESSES[dist]))
        else:
            dist_sensors.append(())

# ----------------------- Tests -----------------------

MOTOR_MAX_SPEED =   10000

def test_motors():
    print("-- Testing all motors simultaniously")
    print("forward")
    simple_turn_all_motors(speed=MOTOR_MAX_SPEED)
    time.sleep(2)

    print("stop")
    simple_turn_all_motors(speed=0)
    time.sleep(1)

    print("backward")
    simple_turn_all_motors(speed=MOTOR_MAX_SPEED, forward=False)
    time.sleep(2)

    print("stop")
    simple_turn_all_motors(speed=0)
    time.sleep(1)

def test_single_motor(m):
    print("-- Testing motor " + str(m))
    if not MOTORS_ENABLED[m]:
        print("motor disabled")
        return
    pins = motor_pwm_pins[m]
    print("forward")
    turn_motor(pins, speed=MOTOR_MAX_SPEED)
    time.sleep(2)

    print("stop")
    turn_motor(pins, speed=0)
    time.sleep(1)

    print("backward")
    turn_motor(pins, speed=MOTOR_MAX_SPEED, forward=False)
    time.sleep(2)

    print("stop")
    turn_motor(pins, speed=0)
    time.sleep(1)

def test_encoder(e):
    print("-- Testing encoder " + str(e))
    if not ENCODERS_ENABLED[e]:
        print("encoder disabled")
        return
    s_time = time.time()
    last_position = None
    while time.time() - s_time < 5:
        position = encoders[e].position
        if last_position == None or position != last_position:
            print("New position: " + str(position))
        last_position = position

def test_dist_sensor(d):
    print("-- Testing distance sensor " + str(d))
    if not DIST_ENABLED[d]:
        print("distance sensor disabled")
        return
    s_time = time.time()
    last_position = None
    while time.time() - s_time < 10:
        position = dist_sensors[d].range
        if last_position == None or position != last_position:
            print("New distance: " + str(position))
        last_position = position

if I2C_ENABLED:
    i2c_dist_address_setup()
