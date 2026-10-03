# riscv-humility

A RISC-V port of [Humility](https://github.com/oxidecomputer/humility), the Hubris
debugger, to use with
[riscv-hubris](https://github.com/sarvesh-bodakhe/riscv-hubris).

| Chip                   | Status      |
|------------------------|-------------|
| ESP32-C6               | Supported   |
| ESP32-C3               | In progress |
| RP2350 (Hazard3 core)  | In progress |

On the ESP32-C6, over its built-in USB-Serial-JTAG, it flashes images,
reads tasks, memory, registers, stacks and ring buffers, takes dumps, and
runs the Hubris test suite.
