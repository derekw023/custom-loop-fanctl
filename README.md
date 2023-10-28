# PC thermal management system build on an RP2040
This set of crates implements a thermal management system suitable for high performance PCs. The current model is very specific to the use case of a 10k NTC thermistor and PWM control outputs for fans but this is in the process of being expanded upon. 

## Controller firmware
Bare metal Rust targeting the RP2040. Controllable by USB

## Configuration and Monitoring Interface
USB CDC serial console, accessible by ???. Working on this...