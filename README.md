# xdcc-cli 

xdcc-cli is a small tool written in Rust and tokio to download files using the [XDCC](https://en.wikipedia.org/wiki/XDCC) protocol.

## Example Usage

1. Visit a site like https://nibl.co.uk/ and copy a command (batch mode is also supported).
2. Run the tool using the prebuilt binary
   ```bash
   xdcc-cli "/msg {bot} xdcc send {pack}"
   ```
   or from source
   ```bash
   cargo run -- "/msg {bot} xdcc send {pack}"
   ```
3. When the download is finished, the tool prints the downloaded file name, which can be used with pipes on UNIX systems. 

## Detailed Usage
```
Usage: xdcc-cli [OPTIONS] <COMMAND>

Arguments:
<COMMAND>

Options:
-s, --server <SERVER>                    [default: irc.rizon.net:6667]
-c, --channel <CHANNEL>                  [default: #nibl]
-n, --nickname <NICKNAME>                [default: xdcc-cli]
-t, --timeout-seconds <TIMEOUT_SECONDS>  [default: 30]
-h, --help                               Print help
-V, --version                            Print version
```
