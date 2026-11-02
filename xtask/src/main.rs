use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask", about = "LaserTargets workspace automation & Pi deployment CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Cross-compile Linux ARM64 (Raspberry Pi 4) artifacts in Docker container on remote build server
    BuildArm64 {
        /// Cargo build profile
        #[arg(long, default_value = "rpi-dev")]
        profile: Profile,

        /// Remote Docker build server (defaults to dedicated build host)
        #[arg(long, default_value = "joel@192.168.1.110")]
        remote: String,

        /// Do not use Docker build cache
        #[arg(long)]
        no_cache: bool,

        /// Skip exporting artifacts to dist/pi
        #[arg(long)]
        no_export: bool,

        /// Print commands without executing them
        #[arg(long)]
        dry_run: bool,
    },

    /// Check Raspberry Pi network reachability, SSH, and hardware status
    Check {
        /// Raspberry Pi host or IP
        #[arg(long, default_value = "lasertargets.local")]
        host: String,

        /// SSH username
        #[arg(long, default_value = "lasertargets")]
        user: String,
    },

    /// Apply OS performance tweaks (CPU governor, disable USB autosuspend)
    Optimize {
        /// Raspberry Pi host or IP
        #[arg(long, default_value = "lasertargets.local")]
        host: String,

        /// SSH username
        #[arg(long, default_value = "lasertargets")]
        user: String,
    },

    /// Deploy server, libraries, assets, and systemd service to Raspberry Pi
    Deploy {
        /// Raspberry Pi host or IP
        #[arg(long, default_value = "lasertargets.local")]
        host: String,

        /// SSH username
        #[arg(long, default_value = "lasertargets")]
        user: String,

        /// Build ARM64 artifacts before deploying
        #[arg(long)]
        build: bool,

        /// Start background systemd service after deployment (default: leave stopped for interactive testing)
        #[arg(long)]
        start: bool,
    },

    /// Run the server interactively on the Pi over SSH with live logs
    Run {
        /// Raspberry Pi host or IP
        #[arg(long, default_value = "lasertargets.local")]
        host: String,

        /// SSH username
        #[arg(long, default_value = "lasertargets")]
        user: String,

        /// Binary to execute: server or dac-test
        #[arg(long, default_value = "server")]
        bin: String,

        /// Log level (RUST_LOG)
        #[arg(long, default_value = "info")]
        log_level: String,
    },

    /// View or stream systemd journal logs from the Pi
    Logs {
        /// Raspberry Pi host or IP
        #[arg(long, default_value = "lasertargets.local")]
        host: String,

        /// SSH username
        #[arg(long, default_value = "lasertargets")]
        user: String,

        /// Number of recent log lines to display
        #[arg(short = 'n', long, default_value_t = 50)]
        lines: usize,

        /// Stream logs continuously (follow)
        #[arg(short = 'f', long)]
        follow: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Profile {
    RpiDev,
    Release,
}

impl Profile {
    fn as_str(&self) -> &'static str {
        match self {
            Profile::RpiDev => "rpi-dev",
            Profile::Release => "release",
        }
    }
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to locate workspace root")
        .to_path_buf()
}

fn target_spec(user: &str, host: &str) -> String {
    if host.contains('@') {
        host.to_string()
    } else {
        format!("{user}@{host}")
    }
}

fn run_cmd(mut cmd: Command, dry_run: bool) -> Result<(), String> {
    let program = cmd.get_program().to_string_lossy().to_string();
    let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().to_string()).collect();
    let display = format!("{program} {}", args.join(" "));

    if dry_run {
        println!("[DRY-RUN] {display}");
        return Ok(());
    }

    println!("--> {display}");
    let status = cmd.status().map_err(|e| format!("Failed to execute '{display}': {e}"))?;
    if !status.success() {
        return Err(format!("Command exited with status: {status}"));
    }
    Ok(())
}

fn cmd_output(mut cmd: Command) -> Result<String, String> {
    let output = cmd.output().map_err(|e| format!("Failed to run command: {e}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Command failed: {err}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// ---------------------------------------------------------------------------
// Build ARM64
// ---------------------------------------------------------------------------

fn build_arm64(profile: Profile, remote: &str, no_cache: bool, no_export: bool, dry_run: bool) -> Result<(), String> {
    let root = project_root();
    println!("=== Building LaserTargets for Linux ARM64 ({}) ===", profile.as_str());
    println!("Dedicated Build Host: {remote}");
    println!("Dockerfile: docker/Dockerfile.rpi4");

    let local_tar = std::env::temp_dir().join("lasertargets_src.tar");
    let remote_tar = "C:\\Users\\joel\\AppData\\Local\\Temp\\lasertargets_src.tar";
    let remote_build_dir = "C:\\Users\\joel\\AppData\\Local\\Temp\\rpi_build";

    // Step 1: Package source code locally using tar
    println!("\n[1/4] Packaging local source context into tarball...");
    let mut tar_cmd = Command::new("tar");
    tar_cmd.current_dir(&root);
    tar_cmd.args([
        "--exclude=target",
        "--exclude=dist",
        "--exclude=.git",
        "--exclude=*.so",
        "--exclude=*.exe",
        "-cf",
        local_tar.to_str().unwrap(),
        "-C",
        root.to_str().unwrap(),
        ".",
    ]);
    run_cmd(tar_cmd, dry_run)?;

    // Step 2: Transfer source tarball to remote Docker build host
    println!("\n[2/4] Transferring source tarball to {remote}...");
    let mut scp_cmd = Command::new("scp");
    scp_cmd.args([local_tar.to_str().unwrap(), &format!("{remote}:{remote_tar}")]);
    let scp_res = run_cmd(scp_cmd, dry_run);
    let _ = std::fs::remove_file(&local_tar);
    scp_res?;

    // Step 3: Build image using docker/Dockerfile.rpi4 on the remote host and extract artifacts
    println!("\n[3/4] Building Docker artifact image with docker/Dockerfile.rpi4 on {remote}...");
    let no_cache_flag = if no_cache { "--no-cache" } else { "" };
    let remote_script = format!(
        "powershell -Command \"\
            $ErrorActionPreference = 'Stop'; \
            Remove-Item -Recurse -Force {remote_build_dir} -ErrorAction SilentlyContinue; \
            New-Item -ItemType Directory -Force -Path {remote_build_dir}\\dist | Out-Null; \
            Set-Location {remote_build_dir}; \
            tar -xf {remote_tar}; \
            Remove-Item {remote_tar} -Force -ErrorAction SilentlyContinue; \
            $env:DOCKER_BUILDKIT = '1'; \
            Write-Host '--> docker build with docker/Dockerfile.rpi4...' -ForegroundColor Cyan; \
            docker build {no_cache_flag} -f docker/Dockerfile.rpi4 -t lasertargets-server-rpi4-artifact:remote \
                --build-arg BASE_IMAGE=lasertargets-cross-aarch64 \
                --build-arg TARGET_TRIPLE=aarch64-unknown-linux-gnu \
                --build-arg CARGO_PROFILE={} .; \
            if ($LASTEXITCODE -ne 0) {{ exit $LASTEXITCODE }}; \
            Write-Host '--> Extracting /dist from container image...' -ForegroundColor Cyan; \
            $cid = docker create lasertargets-server-rpi4-artifact:remote; \
            try {{ docker cp `${{cid}}:/dist/. {remote_build_dir}\\dist\\ }} finally {{ docker rm -f $cid | Out-Null }}\
        \"",
        profile.as_str()
    );

    let mut build_cmd = Command::new("ssh");
    build_cmd.args(["-t", remote, &remote_script]);
    run_cmd(build_cmd, dry_run)?;

    // Step 4: Export artifacts to dist/pi
    if !no_export {
        println!("\n[4/4] Fetching compiled ARM64 artifacts to dist/pi...");
        let dist = root.join("dist").join("pi");
        if !dist.exists() {
            std::fs::create_dir_all(&dist).map_err(|e| format!("Failed to create dist/pi: {e}"))?;
        }

        let artifacts = ["server", "dac-test", "libHeliosLaserDAC.so"];
        for artifact in artifacts {
            let remote_path = format!("{remote_build_dir}\\dist\\{artifact}");
            let local_path = dist.join(artifact);
            if dry_run {
                println!("[DRY-RUN] scp {remote}:{remote_path} {}", local_path.display());
                continue;
            }
            let mut fetch_cmd = Command::new("scp");
            fetch_cmd.args([&format!("{remote}:{remote_path}"), local_path.to_str().unwrap()]);
            let _ = fetch_cmd.status();
        }

        println!("Build complete! Artifacts exported to: {}", dist.display());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Check Pi
// ---------------------------------------------------------------------------

fn check_pi(user: &str, host: &str) -> Result<(), String> {
    let target = target_spec(user, host);
    println!("=== Checking Raspberry Pi Status ({target}) ===");

    // 1. Ping
    println!("\n--- 1. Network Ping ---");
    let ping_cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("ping");
        c.args(["-n", "2", host]);
        c
    } else {
        let mut c = Command::new("ping");
        c.args(["-c", "2", host]);
        c
    };
    match cmd_output(ping_cmd) {
        Ok(_) => println!("  [PASS] Ping OK to {host}"),
        Err(_) => println!("  [WARN] Ping failed or timed out for {host}"),
    }

    // 2. SSH Connection
    println!("\n--- 2. SSH Connectivity ---");
    let mut ssh_test = Command::new("ssh");
    ssh_test.args([
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=5",
        &target,
        "echo OK",
    ]);
    match cmd_output(ssh_test) {
        Ok(out) if out.contains("OK") => println!("  [PASS] SSH login successful"),
        Ok(_) | Err(_) => {
            return Err(format!("Cannot establish SSH connection to {target}. Check host, user, and SSH keys."));
        }
    }

    // 3. System info
    println!("\n--- 3. System Information ---");
    let mut uname_cmd = Command::new("ssh");
    uname_cmd.args([&target, "uname -srm; uptime -p"]);
    if let Ok(info) = cmd_output(uname_cmd) {
        for line in info.lines() {
            println!("  [INFO] {line}");
        }
    }

    // 4. CPU Governor
    println!("\n--- 4. CPU Governor ---");
    let mut gov_cmd = Command::new("ssh");
    gov_cmd.args([&target, "cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo unknown"]);
    if let Ok(gov) = cmd_output(gov_cmd) {
        if gov == "performance" {
            println!("  [PASS] CPU Governor is '{gov}'");
        } else {
            println!("  [WARN] CPU Governor is '{gov}' (run 'cargo xtask optimize' to set to 'performance')");
        }
    }

    // 5. USB DAC hardware
    println!("\n--- 5. USB DAC Hardware ---");
    let mut lsusb_cmd = Command::new("ssh");
    lsusb_cmd.args([&target, "lsusb"]);
    if let Ok(devices) = cmd_output(lsusb_cmd) {
        let dac_found = devices.lines().any(|l| {
            let lower = l.to_lowercase();
            lower.contains("helios") || lower.contains("laser") || lower.contains("ftdi") || lower.contains("1105")
        });
        if dac_found {
            println!("  [PASS] Laser DAC USB hardware detected");
        } else {
            println!("  [WARN] No known Laser DAC detected in 'lsusb'");
        }
    }

    // 6. LaserTargets service status
    println!("\n--- 6. Service Status ---");
    let mut svc_cmd = Command::new("ssh");
    svc_cmd.args([&target, "systemctl is-active lasertargets-server 2>/dev/null || echo inactive"]);
    if let Ok(status) = cmd_output(svc_cmd) {
        println!("  [INFO] lasertargets-server.service: {status}");
    }

    println!("\nCheck complete.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Optimize Pi
// ---------------------------------------------------------------------------

fn optimize_pi(user: &str, host: &str) -> Result<(), String> {
    let target = target_spec(user, host);
    println!("=== Optimizing Raspberry Pi ({target}) ===");

    // 1. Set CPU governor to performance
    println!("\n[1/3] Setting CPU governor to 'performance'...");
    let mut gov_cmd = Command::new("ssh");
    gov_cmd.args([
        &target,
        "echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor >/dev/null 2>&1 || true",
    ]);
    run_cmd(gov_cmd, false)?;

    // 2. Persistent CPU governor via systemd
    println!("\n[2/3] Setting up persistent CPU performance systemd service...");
    let service_content = r#"[Unit]
Description=Set CPU Scaling Governor to Performance
After=sys-devices-system-cpu-cpu0-cpufreq-scaling_governor.device

[Service]
Type=oneshot
ExecStart=/bin/sh -c "echo performance | tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor"

[Install]
WantedBy=multi-user.target
"#;

    let write_service = format!(
        "sudo bash -c 'cat > /etc/systemd/system/cpu-performance.service << \"EOF\"\n{service_content}EOF\n' && sudo systemctl daemon-reload && sudo systemctl enable --now cpu-performance.service"
    );
    let mut svc_cmd = Command::new("ssh");
    svc_cmd.args([&target, &write_service]);
    run_cmd(svc_cmd, false)?;

    // 3. Disable USB autosuspend
    println!("\n[3/3] Disabling USB autosuspend...");
    let autosuspend_script = r#"
echo -1 | sudo tee /sys/module/usbcore/parameters/autosuspend >/dev/null 2>&1 || true

CMDLINE=""
if [ -f /boot/firmware/cmdline.txt ]; then
    CMDLINE="/boot/firmware/cmdline.txt"
elif [ -f /boot/cmdline.txt ]; then
    CMDLINE="/boot/cmdline.txt"
fi

if [ -n "$CMDLINE" ]; then
    if ! grep -q "usbcore.autosuspend=-1" "$CMDLINE"; then
        sudo sed -i 's/$/ usbcore.autosuspend=-1/' "$CMDLINE"
        echo "Added usbcore.autosuspend=-1 to $CMDLINE (takes effect after reboot)"
    else
        echo "usbcore.autosuspend=-1 already present in $CMDLINE"
    fi
fi
"#;
    let mut usb_cmd = Command::new("ssh");
    usb_cmd.args([&target, autosuspend_script]);
    run_cmd(usb_cmd, false)?;

    println!("\nOptimization complete!");
    Ok(())
}

// ---------------------------------------------------------------------------
// Deploy Pi
// ---------------------------------------------------------------------------

fn deploy_pi(user: &str, host: &str, build: bool, start: bool) -> Result<(), String> {
    let target = target_spec(user, host);
    let root = project_root();
    let dist = root.join("dist").join("pi");
    let server_bin = dist.join("server");
    let helios_lib = dist.join("libHeliosLaserDAC.so");
    let dac_test_bin = dist.join("dac-test");

    if build {
        build_arm64(Profile::RpiDev, "joel@192.168.1.110", false, false, false)?;
    }

    if !server_bin.exists() {
        return Err(format!(
            "Server artifact not found at {}. Run 'cargo xtask build-arm64' or pass '--build'.",
            server_bin.display()
        ));
    }

    println!("=== Deploying LaserTargets to {target} ===");

    // 1. Stop existing service
    println!("\n--- 1. Stopping existing service ---");
    let mut stop_cmd = Command::new("ssh");
    stop_cmd.args([&target, "sudo systemctl stop lasertargets-server 2>/dev/null || true"]);
    let _ = stop_cmd.status();

    // 2. Directories
    println!("\n--- 2. Ensuring directory layout ---");
    let mut dir_cmd = Command::new("ssh");
    dir_cmd.args([
        &target,
        &format!("sudo mkdir -p /opt/lasertargets/lib /opt/lasertargets/stats && sudo chown -R {user}:{user} /opt/lasertargets"),
    ]);
    run_cmd(dir_cmd, false)?;

    // 3. Deploy server binary
    println!("\n--- 3. Transferring server binary ---");
    let mut scp_server = Command::new("scp");
    scp_server.args([server_bin.to_str().unwrap(), &format!("{target}:/opt/lasertargets/server")]);
    run_cmd(scp_server, false)?;

    let mut perm_cmd = Command::new("ssh");
    perm_cmd.args([
        &target,
        "chmod +x /opt/lasertargets/server && sudo setcap cap_sys_nice+eip /opt/lasertargets/server 2>/dev/null || true",
    ]);
    run_cmd(perm_cmd, false)?;

    // 4. Deploy dac-test if present
    if dac_test_bin.exists() {
        println!("\n--- 4. Transferring dac-test ---");
        let mut scp_dactest = Command::new("scp");
        scp_dactest.args([dac_test_bin.to_str().unwrap(), &format!("{target}:/opt/lasertargets/dac-test")]);
        let _ = scp_dactest.status();
        let mut perm_dt = Command::new("ssh");
        perm_dt.args([&target, "chmod +x /opt/lasertargets/dac-test"]);
        let _ = perm_dt.status();
    }

    // 5. Deploy shared library if present
    if helios_lib.exists() {
        println!("\n--- 5. Transferring libHeliosLaserDAC.so ---");
        let mut scp_lib = Command::new("scp");
        scp_lib.args([helios_lib.to_str().unwrap(), &format!("{target}:/opt/lasertargets/lib/")]);
        run_cmd(scp_lib, false)?;

        let mut scp_lib2 = Command::new("scp");
        scp_lib2.args([helios_lib.to_str().unwrap(), &format!("{target}:/opt/lasertargets/")]);
        let _ = scp_lib2.status();

        let mut cp_usr_lib = Command::new("ssh");
        cp_usr_lib.args([
            &target,
            "sudo cp /opt/lasertargets/libHeliosLaserDAC.so /usr/lib/libHeliosLaserDAC.so 2>/dev/null || true",
        ]);
        let _ = cp_usr_lib.status();
    }

    // 6. Deploy assets folder if present
    let assets = root.join("assets");
    if assets.exists() {
        println!("\n--- 6. Transferring assets ---");
        let mut scp_assets = Command::new("scp");
        scp_assets.args(["-r", assets.to_str().unwrap(), &format!("{target}:/opt/lasertargets/")]);
        let _ = scp_assets.status();
    }

    // 7. Deploy systemd unit file
    let service_file = root.join("deploy").join("lasertargets-server.service");
    if service_file.exists() {
        println!("\n--- 7. Installing systemd unit ---");
        let mut scp_svc = Command::new("scp");
        scp_svc.args([service_file.to_str().unwrap(), &format!("{target}:/tmp/lasertargets-server.service")]);
        run_cmd(scp_svc, false)?;

        let mut mv_svc = Command::new("ssh");
        mv_svc.args([
            &target,
            "sudo mv /tmp/lasertargets-server.service /etc/systemd/system/ && sudo systemctl daemon-reload",
        ]);
        run_cmd(mv_svc, false)?;
    }

    // 8. Deploy logrotate configuration
    let logrotate_file = root.join("deploy").join("lasertargets-logrotate");
    if logrotate_file.exists() {
        println!("\n--- 8. Installing logrotate configuration ---");
        let mut scp_lr = Command::new("scp");
        scp_lr.args([logrotate_file.to_str().unwrap(), &format!("{target}:/tmp/lasertargets-logrotate")]);
        let _ = scp_lr.status();

        let mut mv_lr = Command::new("ssh");
        mv_lr.args([
            &target,
            "sudo mv /tmp/lasertargets-logrotate /etc/logrotate.d/lasertargets && sudo chown root:root /etc/logrotate.d/lasertargets && sudo chmod 644 /etc/logrotate.d/lasertargets",
        ]);
        let _ = mv_lr.status();
    }

    // 9. Start or leave stopped
    if start {
        println!("\n--- 9. Enabling and starting lasertargets-server service ---");
        let mut start_cmd = Command::new("ssh");
        start_cmd.args([&target, "sudo systemctl enable --now lasertargets-server"]);
        run_cmd(start_cmd, false)?;
    } else {
        println!("\n--- 9. Keeping background service stopped (frees USB DAC for interactive run) ---");
        let mut stop_bg = Command::new("ssh");
        stop_bg.args([
            &target,
            "sudo systemctl disable --now lasertargets-server 2>/dev/null; sudo killall -9 server 2>/dev/null || true",
        ]);
        let _ = stop_bg.status();
    }

    println!("\n=== Deployment complete! ===");
    println!("Run interactively with live logs: cargo xtask run");
    println!("Or start as background service:    ssh {target} 'sudo systemctl start lasertargets-server'");

    Ok(())
}

// ---------------------------------------------------------------------------
// Run Server Interactively
// ---------------------------------------------------------------------------

fn run_pi(user: &str, host: &str, bin: &str, log_level: &str) -> Result<(), String> {
    let target = target_spec(user, host);
    println!("=== Running {bin} interactively on {target} ===");

    println!("Ensuring background service is stopped to free USB DAC...");
    let mut stop_cmd = Command::new("ssh");
    stop_cmd.args([
        &target,
        "sudo systemctl stop lasertargets-server 2>/dev/null; sudo killall -9 server 2>/dev/null || true",
    ]);
    let _ = stop_cmd.status();

    println!("Launching {bin} (RUST_LOG={log_level}). Press Ctrl+C to stop.\n");
    let mut run_cmd = Command::new("ssh");
    run_cmd.args([
        "-t",
        &target,
        &format!("cd /opt/lasertargets && sudo env LD_LIBRARY_PATH=/opt/lasertargets RUST_LOG={log_level} /opt/lasertargets/{bin}"),
    ]);

    let _ = run_cmd.status();
    println!("\nInteractive session closed.");
    Ok(())
}

// ---------------------------------------------------------------------------
// View Logs
// ---------------------------------------------------------------------------

fn logs_pi(user: &str, host: &str, lines: usize, follow: bool) -> Result<(), String> {
    let target = target_spec(user, host);
    let mut log_cmd = Command::new("ssh");
    let mut cmd_str = format!("journalctl -u lasertargets-server -n {lines} --no-pager");
    if follow {
        cmd_str.push_str(" -f");
    }
    log_cmd.args([&target, &cmd_str]);
    let _ = log_cmd.status();
    Ok(())
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();
    let res = match cli.command {
        Commands::BuildArm64 {
            profile,
            remote,
            no_cache,
            no_export,
            dry_run,
        } => build_arm64(profile, &remote, no_cache, no_export, dry_run),
        Commands::Check { host, user } => check_pi(&user, &host),
        Commands::Optimize { host, user } => optimize_pi(&user, &host),
        Commands::Deploy {
            host,
            user,
            build,
            start,
        } => deploy_pi(&user, &host, build, start),
        Commands::Run {
            host,
            user,
            bin,
            log_level,
        } => run_pi(&user, &host, &bin, &log_level),
        Commands::Logs {
            host,
            user,
            lines,
            follow,
        } => logs_pi(&user, &host, lines, follow),
    };

    if let Err(e) = res {
        eprintln!("\nError: {e}");
        std::process::exit(1);
    }
}
