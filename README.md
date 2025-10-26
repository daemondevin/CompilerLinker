# CompilerLinker

A fast, lightweight command-line utility for creating Windows file system links with colored output and clear error messages.

## Features

- Create junctions, symbolic links, hard links, and soft links
- Colorized output for better readability
- Zero runtime dependencies
- Clear error messages with specific exit codes
- Single executable (~500KB)

## Installation

### From Source

```bash
git clone https://github.com/yourusername/CompilerLinker.git
cd CompilerLinker
cargo build --release
```

The compiled binary will be at `target/release/CompilerLinker.exe`

### Pre-built Binaries

Download the latest release from the [Releases](https://github.com/daemondevin/CompilerLinker/releases) page.

## Usage

```bash
CompilerLinker [OPTIONS] --link <PATH> --target <PATH>
```

### Options

- `-t, --link <PATH>` - Source path where the link will be created
- `-o, --target <PATH>` - Destination path the link points to
- `-j, --junction` - Create a junction point
- `-s, --soft` - Create a soft link (directory symlink)
- `-d, --symbolic` - Create a symbolic link (file symlink)
- `-h, --hard` - Create a hard link

You must specify **exactly one** link type.

## Examples

### Create a Junction
```bash
CompilerLinker -j --link "C:\MyLink" --target "C:\MyTarget"
```
- ✅ Same volume only
- ✅ No admin privileges required
- ✅ Works for directories

### Create a Soft Link (Directory Symlink)
```bash
CompilerLinker -s --link "D:\MyLink" --target "C:\MyTarget"
```
- ✅ Works across different volumes
- ⚠️ Requires administrator privileges
- ✅ Works for directories

### Create a Symbolic Link (File Symlink)
```bash
CompilerLinker -d --link "C:\link.txt" --target "C:\target.txt"
```
- ✅ Works across different volumes
- ⚠️ Requires administrator privileges
- ✅ Works for files only

### Create a Hard Link
```bash
CompilerLinker -h --link "C:\link.txt" --target "C:\target.txt"
```
- ✅ Same volume only
- ✅ No admin privileges required
- ✅ Works for files only

## Link Types Comparison

| Type | Cross-Volume | Admin Required | Files | Directories |
|------|--------------|----------------|-------|-------------|
| Junction | ❌ | ❌ | ❌ | ✅ |
| Soft Link | ✅ | ✅ | ❌ | ✅ |
| Symbolic Link | ✅ | ✅ | ✅ | ❌ |
| Hard Link | ❌ | ❌ | ✅ | ❌ |

## Exit Codes

- `0` - Success
- `1` - No link type or multiple types specified
- `2` - Junction creation failed
- `3` - Symbolic link creation failed
- `4` - Hard link creation failed
- `5` - Soft link creation failed
- `7` - Not running on Windows

## Requirements

- Windows Vista or later
- Administrator privileges for symbolic and soft links (unless Developer Mode is enabled)

## Developer Mode

To create symlinks without admin privileges, enable Windows Developer Mode:

1. Open **Settings** → **Update & Security** → **For developers**
2. Enable **Developer Mode**

## Building from Source

### Prerequisites

- Rust 1.70 or later
- Windows SDK (for the build script)

### Build

```bash
cargo build --release
```

### Dependencies

- `owo-colors` - Terminal color output
- `structopt` - Command-line argument parsing
- `winresource` (build-only) - Embed version info and icon

## Use in NSIS Installers

```nsis
RequestExecutionLevel admin

Section "Install"
    ; Create a soft link across volumes
    nsExec::ExecToStack `"$INSTDIR\CompilerLinker.exe" -s --link "D:\link" --target "C:\target"`
    Pop $0  ; Exit code
    Pop $1  ; Output
    
    ${If} $0 == "0"
        DetailPrint "Link created successfully"
    ${Else}
        MessageBox MB_OK "Failed to create link (code: $0)"
    ${EndIf}
SectionEnd
```

## License

MIT License - see [LICENSE](LICENSE) for details

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- Uses Windows native APIs for junction creation
- Built with Rust for performance and safety
