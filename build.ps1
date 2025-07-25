# 定义输出目录

# 创建输出目录

# 备份原始的 .cargo/config.toml（如果存在）
$configPath = ".cargo/config.toml"
$backupPath = ".cargo/config.toml.bak"
if (Test-Path $configPath) {
    Copy-Item $configPath $backupPath -Force
}

# 第一次编译：不使用 config.toml
if (Test-Path $configPath) {
    Write-Host "Removing $configPath"
    Remove-Item $configPath
}
Write-Host "Starting default build..."
cargo build --target i686-pc-windows-msvc --profile release
if ($?) {
    Copy-Item "target/i686-pc-windows-msvc/release/tradeServer.exe" -Destination "./tradeServer.exe" -Force
}

# 第二次编译：使用 windows subsystem 配置
$configContent = @"
[build]
target = "i686-pc-windows-msvc"  
[target.i686-pc-windows-msvc]
rustflags = ["-C", "link-args=/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup"]
"@
Set-Content $configPath $configContent
Write-Host "Starting Windows subsystem build..."
cargo clean
cargo build --target i686-pc-windows-msvc --profile release
if ($?) {
    Copy-Item "target/i686-pc-windows-msvc/release/tradeServer.exe" -Destination "./tradeServerW.exe" -Force
}

# 恢复原始配置文件
if (Test-Path $backupPath) {
    Move-Item $backupPath $configPath -Force
} else {
    Remove-Item $configPath
}

Write-Host "Build completed!"
Write-Host "Default version location: tradeServer.exe"
Write-Host "Windows subsystem version location: tradeServerW.exe" 