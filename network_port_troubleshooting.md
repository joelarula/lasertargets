# Network and Port Troubleshooting Guide

## 1. Verify Server is Listening
- On the server, run:
  ```sh
  netstat -an | findstr 6000
  ```
- You should see:
  ```
  UDP    [::]:6000              *:*
  ```
- This means the server is listening on all interfaces for UDP port 6000.

## 2. Check Server IP Address
- On the server, run:
  ```sh
  ipconfig
  ```
- Use the IPv4 address of the adapter connected to your LAN (e.g., 192.168.1.110).

## 3. Test Connectivity from Client
- Ping the server from the client:
  ```sh
  ping 192.168.1.110
  ```
- Scan the UDP port from the client (requires nmap):
  ```sh
  nmap -sU -p 6000 192.168.1.110
  ```
- Result meanings:
  - `open|filtered`: Port is reachable, but UDP may not respond to probes (normal for custom protocols).
  - `closed`: Port is not reachable.

## 4. Firewall Configuration
- On the server, open UDP port 6000:
  ```powershell
  New-NetFirewallRule -DisplayName "Allow UDP 6000" -Direction Inbound -Protocol UDP -LocalPort 6000 -Action Allow
  ```
- Verify the rule:
  ```powershell
  Get-NetFirewallRule | findstr 6000
  ```

## 5. Client Configuration
- Ensure the client is configured to use the correct server IP and port.
- Both client and server must use the same protocol (UDP/QUIC) and compatible versions.

## 6. Additional Checks
- Try running the client on the same machine as the server using `127.0.0.1` to rule out network issues.
- Check server and client logs for connection attempts, errors, or warnings.
- If using a router or VPN, ensure port forwarding or routing is correct.

## 7. Useful Commands
- List all drives:
  - Command Prompt: `wmic logicaldisk get name`
  - PowerShell: `Get-PSDrive -PSProvider FileSystem`
- Map a network drive:
  ```sh
  net use Z: \\192.168.1.110\lasertargets\target\debug
  ```

## 8. Notes
- UDP ports may show as `open|filtered` in nmap even if everything is working.
- Custom protocols may not respond to generic UDP probes.
- Always check both firewall and application logs for troubleshooting.
