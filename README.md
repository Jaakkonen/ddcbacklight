# DDC Backlight

A command-line tool for controlling external monitor brightness using the DDC/CI protocol. Supports both Intel and AMD GPUs with automatic I2C device detection.

## Commands

```bash
# Get current brightness
ddcbacklight get-brightness

# Set brightness to 75%
ddcbacklight set-brightness 75

# Increase brightness by 10%
ddcbacklight set-brightness +10

# Decrease brightness by 5%
ddcbacklight set-brightness -5

# Use specific I2C device
ddcbacklight --i2c-path /dev/i2c-7 get-brightness
```

## Permissions Setup

To use without sudo, grant your user access to I2C devices:

```bash
# Find your I2C device first
sudo ddcbacklight get-brightness  # This will show which device is detected

# Give yourself read/write access (replace i2c-7 with your device)
sudo chmod 666 /dev/i2c-7
```
