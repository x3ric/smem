# smem

**smem** is a memory scanning and visualization tool built in Rust, utilizing `eframe` and `egui` for a modern graphical interface. It allows users to attach to processes, scan memory regions, and visualize or modify memory values in real time.

## Features  

### **Memory Operations**  
- **Attach/Detach**: Connect or disconnect from a process memory using `/proc/<pid>/mem`.  
- **Read/Write**: Directly access and modify memory regions.  
- **Visualization**: Dynamically display and interact with memory regions.  

### **Scanning Functions**  
- **Exact Match**: Search for specific numerical values.  
- **Value Changes**: Detect modifications in memory values.  
- **Trends**: Identify increasing or decreasing value patterns.  

### **User Interface and Controls**  
- **Clipboard Interaction**: Right-click on a memory address to copy it.  
- **Functional Keys**:  
  - **F1**: Attach or detach to a process by ID.  
  - **F2**: Start a memory scan.  
  - **F3**: View previous scan results.  
  - **F4**: Reset scan configurations.  
  - **F5**: Focus on a specific memory address.  
  - **F7**: Lock and continuously update a memory address.  
  - **F8-F10**: Trigger scans for changes, increases, or decreases.  
  - **F11/F12**: Adjust the visualization scale.  

### **Input Value Parsing**  
**smem** supports prefixes for type-specific parsing:  
- **`bool:<value>`**: Boolean, e.g., `bool:true`.  
- **`byte:<value>`**: 8-bit unsigned integer (`UInt8`), e.g., `byte:255`.  
- **`hex:<value>`**: Hexadecimal, auto-converted to the smallest unsigned type, e.g., `hex:FF`.  
- **`int8:<value>`**, **`int16:<value>`**, **`int32:<value>`**, **`int64:<value>`**: Signed integers, e.g., `int16:32767`.  
- **`float32:<value>`**, **`float64:<value>`**: Floating-point numbers, e.g., `float32:3.14`.  
- **`size:<value>`**: Platform-specific unsigned size, e.g., `size:1024`.  
- **`ptr:<value>`**: Pointer, e.g., `ptr:0x1000`.  

**Default (`<value>`)**: If no prefix is provided, values are parsed as signed integers (`Int8`, `Int16`, `Int32`, `Int64`) or floats (`Float32`, `Float64`):  
- `42`  
- `3.14159`  

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
