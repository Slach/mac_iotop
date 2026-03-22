# mac_iotop

A simple macOS disk I/O monitor — like `iotop` for macOS.

Shows per-file read/write rates aggregated per second, based on `fs_usage` output.

## Usage

Requires `sudo` (because `fs_usage` needs root privileges):

```bash
sudo mac_iotop
```

### Example output

```
TIME       | READ/s     | WRITE/s    | PROCESS: FILE
------------------------------------------------------------------------------------------
19:09:02   | 0          | 16.00 KB/s | bun.42029: /Users/user/Library/Caches/app/log.jsonl
19:09:02   | 4.00 KB/s  | 0          | Yandex Helper.8420: /Users/user/Library/Caches/Yandex/...
```

### Debug mode

```bash
sudo DEBUG=1 mac_iotop
```

## Installation

### Via Homebrew

```bash
brew install slach/tap/mac_iotop
```

Then run:

```bash
sudo mac_iotop
```

### From releases

Download the binary for your architecture from [Releases](https://github.com/Slach/mac_iotop/releases):

- `mac_iotop-arm64` — Apple Silicon (M1/M2/M3/M4)
- `mac_iotop-amd64` — Intel Macs

```bash
chmod +x mac_iotop-arm64
sudo ./mac_iotop-arm64
```

### From source

```bash
cargo build --release
sudo ./target/release/mac_iotop
```

## How it works

- Runs `fs_usage -w -f filesys` under the hood
- Filters `RdData` / `WrData` events (actual disk reads/writes with file paths)
- Aggregates bytes per file per second
- Displays human-readable rates (B/s, KB/s, MB/s, GB/s)

## Requirements

- macOS (uses `fs_usage` which is macOS-specific)
- Root privileges (`sudo`)
