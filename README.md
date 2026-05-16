# RES Format Viewer 🖼️

The official, lightning-fast native viewer for the `.RES` image format. Built from the ground up in pure Rust to guarantee zero-bloat, instant launch times, and flawless memory safety.

Available natively for Windows 10/11 (WinUI/DirectX) and Linux (GTK4/GNOME).

## 🚀 Getting Started

The `.RES` format is a streamlined image container. To interact with these files, you have two main tools at your disposal:

### 1. Web Converter (Universal)
You can convert standard images to `.RES` and vice versa directly in your browser. All processing happens locally on your machine, no data is uploaded.

🔗 **[res-converter.vercel.app](https://res-converter.vercel.app/)**
* Drag and drop `JPG`, `PNG`, or `WEBP` to encode to `.RES`.
* Drag and drop `.RES` to decode back to standard formats.
* 100% GDPR compliant and offline-capable.

### 2. Native Windows Viewer
The ultimate way to experience `.RES` files on Windows. 
* **Native Speed:** Bypasses heavy web wrappers like Electron. Maps pixels directly via Windows GDI.
* **Seamless Integration:** Automatically sets itself as the default app for `.RES` files.
* **Zero Config:** Double-click a file, and it instantly opens perfectly fitted to your window.

**Installation:**
Download the latest `RES_Viewer_Setup.exe` from the Releases tab, run it, and follow the setup wizard.

### 3. Native Linux Viewer (Flathub)
A GTK4 application for the Linux desktop, featuring kinetic scrolling, fractional zooming (tbh this is kinda broken atm), and deep OS-level MIME type integration.

*(Currently pending Flathub review. Installation instructions coming soon!)*

---

## 🛠️ Build from Source (Developers)

Ensure you have Rust installed.

**Windows (MSVC Toolchain Required):**
```bash
cargo run --release -p res_winui
```

**Linux (GTK4 Required):**
```bash
cargo run --release -p res_viewer
```
