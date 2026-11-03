# Area Selector Accessibility App

## 🌐 Description

This application is designed to assist video game players with limited field of view or visual accessibility needs.
It enables precise selection of any region on the screen and magnifies it through an adjustable, accessibility-friendly zoom window (“Lupa”).

The tool allows users to reposition and resize the magnified area, control zoom intensity, and toggle passthrough mode (allowing clicks to pass through the magnifier).
All settings are saved automatically, providing a flexible and customizable interface tailored to visually impaired users.
Its modular multi-binary architecture ensures smooth operation, whether selecting areas, magnifying content, or adjusting configurations.

![Description gif image](./assets/area-selector-gif.gif)

---

### 🦉 How a Player With Low Vision Sees the Game
![Description gif image](./assets/tunnel-vision.jpg)

---

## 🦀 Requirements to Run the Project

- **Backend / Core Application:** Rust (2021 Edition)  
- **Frontend / UI:** egui + eframe

---

### 📥 Cloning the Repository

```bash
git clone https://github.com/jfan-dev/area-selector-accessibility-app.git
```

---

### ▶️ Running the Main App

```bash
# Access the project folder
cd area-selector-accessibility-app/

# Build in debug mode
cargo build

# Run the main window (panel with buttons)
cargo run
```

---

### ▶️ Running the Area Selector

```bash
# Fullscreen tool to select the screen area
cargo run -- seletor
```

---

### ▶️ Running the Magnifier (Lupa)

```bash
# Start the magnifier window with area/zoom/position
cargo run -- lupa <x> <y> <width> <height> [zoom|passthrough] [pos_x pos_y]
```

---

### 🐧 Building From Linux / WSL (Ubuntu)

The app uses the Windows API, so it only runs on Windows — but you can develop and cross-compile it from Linux or WSL2.
On WSL, the resulting `.exe` files can be launched straight from the Linux shell and open on the Windows desktop.

```bash
# 1. Install the MinGW-w64 cross-compiler
sudo apt update && sudo apt install -y build-essential mingw-w64

# 2. Install Rust and the Windows GNU target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add x86_64-pc-windows-gnu

# 3. Build (aliases defined in .cargo/config.toml)
cargo wbuild --release
# → target/x86_64-pc-windows-gnu/release/area_selector.exe

# 4. Run (WSL only — opens on the Windows desktop)
cargo wrun --release

# Checks (tests run as Windows executables through WSL interop)
cargo wtest
cargo wclippy
cargo fmt --check
```

If running from inside the WSL filesystem is slow, copy the release folder to the Windows side
(e.g. `/mnt/c/Users/<you>/area-selector/`) and run `area_selector.exe` from there.

---

### ▶️ Running From Release Build (No Rust Required)

If you prefer not to compile the project, you can run it directly using the pre-built binaries:

1. Download the release ZIP from the [GitHub Releases](https://github.com/jfan-dev/area-selector-accessibility-app/releases) page and extract it.
2. Inside the extracted folder, run the main executable:

   **`area_selector.exe`**  
   Launches the main control panel for selecting areas and starting the magnifier.


### 📌 Auxiliary Binaries

These executables are also available in the same folder:

- **`seletor.exe`**  
  Opens the fullscreen area selector for choosing the capture region.

- **`lupa.exe`**  
  Starts the magnifier window (requires arguments such as position, size, zoom, or passthrough).


### ⚠️ Important

Make sure all `.exe` files remain in the **same directory**,  
as they depend on shared resources and configuration files.

---

## 👑 Demonstration

![Description gif image](./assets/area-selector-gif.gif)

---

### ✍🏻️ Additional Features

| Feature | Preview | Description |
|--------|---------|-------------|
| Area Selection Tool | ![Area Selection Tool image](./assets/area-selector-tool.jpg) | Fullscreen drag-to-select with live preview |
| Magnifier Window | ![Magnifier Window image](./assets/magnifier-window.jpg) | Zoomed region with adjustable zoom factor |
| Passthrough Mode | ![Passthrough Mode image](./assets/passthrough-window.jpg) | Lets clicks pass through the magnifier window |

---

## ✍🏻️ Technology Stack

| Technology | Purpose | Website |
|-----------|----------|---------|
| Rust | Core application and binaries | https://www.rust-lang.org/ |
| eframe / egui | Graphical UI framework | https://github.com/emilk/egui |
| Windows API | Screen capture & window mode control | https://learn.microsoft.com/windows |
| serde / serde_json | Persistent storage for config | https://serde.rs/ |

---

## 🛠 Challenges and Solutions

### 🟥 Problem — Persisting User Configuration  
Keeping area, zoom and position across sessions required a clean persistent solution integrated into user directories.  
**Solution:** implemented `AppConfig` using `serde_json` and stored it under the OS config path.

### 🟩 Problem — Click-Through Mode  
Switching between interactive and passthrough modes required dynamic editing of extended window styles.  
**Solution:** read and reapplied HWND using `WS_EX_TRANSPARENT` plus layered attributes.

### 🟦 Problem — High CPU Usage & Memory Leak During Selection

Originally, the selection overlay (the blue rectangle drawn during area selection) was rendered every frame at ~60 FPS while also capturing screen data using the scrap crate.
This caused unnecessary GPU/CPU usage and a visible memory leak, especially on 4K monitors, because the capture pipeline and rendering loop were being executed far more often than necessary.

**Solution:** replaced the frame-based capture from scrap with the Windows Accessibility API, which provides a more efficient, event-driven way to retrieve screen content. This eliminated the leak, reduced resource usage, and made the selection preview significantly smoother and more stable overall.

---

## 🐛 Known Bugs / Limitations

### - Magnifier window does not automatically update its size or position.
  After adjusting zoom or coordinates in the main app, the magnifier must be restarted manually or the stated toogled between passthrough because the window is not yet reactive to changes in configuration. The window size (inner_size) and position (viewport.position) are only applied when the binary (lupa.exe) launches.

---

## 🐼 Developed by

**Jaime Fernandes**  
📧 jfernan10@gmail.com  
🔗 https://www.linkedin.com/in/jfan-dev
