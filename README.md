# smem

**smem** is a memory scanning and visualization tool written in Rust, utilizing the `eframe` and `egui` crates for a modern graphical user interface. With **smem**, users can attach to processes, scan memory regions, and visualize or modify memory values in real-time.

## Features

- Attach to processes via `/proc/<pid>/mem`.
- Read and write memory regions.
- Scan memory for specific values or changes.
- Interactive visualization of memory regions.
- Advanced scanning modes:
  - Exact match
  - Value changed
  - Value increased or decreased
- Lock memory values to continuously update them.
- Intuitive controls:
  - **Right-click**: Copy memory address.
  - **F1**: Attach to the PID.
  - **F2**: Initiate a scan.
  - **F3**: View previous scan results.
  - **F4**: Reset scan settings.
  - **F5**: Set target address.
  - **F7**: Lock the current address.
  - **F8**: Scan for changed values.
  - **F9**: Scan for increased values.
  - **F10**: Scan for decreased values.
  - **F11**: Decrease visualization size (zoom out).
  - **F12**: Increase visualization size (zoom in).

## Requirements

- **Rust** (1.70 or later recommended)
- Linux (requires `/proc` filesystem support)
- Root permissions (to access process memory)

## Installation

Clone the repository and run the application directly:

```bash
git clone https://github.com/x3ric/smem.git
cd smem
sudo cargo run -- "$(pidof test | awk '{print $1}')"
```

![Image](./img.png)
