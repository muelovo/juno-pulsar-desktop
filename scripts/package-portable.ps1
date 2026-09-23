param([string]$Configuration = "release")
$ErrorActionPreference = "Stop"
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$version = (Get-Content -Raw -LiteralPath (Join-Path $projectRoot "package.json") | ConvertFrom-Json).version
$outputRoot = Join-Path $projectRoot "release"
$stage = Join-Path $outputRoot "Juno-Pulsar-Desktop-$version-windows-x64"
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item -LiteralPath (Join-Path $projectRoot "src-tauri/target/$Configuration/juno-pulsar-desktop.exe") -Destination (Join-Path $stage "Juno Pulsar Desktop.exe")
foreach ($name in @("LICENSE", "README.md", "CHANGELOG.md")) { Copy-Item -LiteralPath (Join-Path $projectRoot $name) -Destination $stage }
Copy-Item -LiteralPath (Join-Path $projectRoot "assets/sample-skin") -Destination $stage -Recurse -Force
New-Item -ItemType Directory -Force -Path (Join-Path $stage "skins") | Out-Null
Copy-Item -LiteralPath (Join-Path $projectRoot "assets/skins/hope-heart-realistic-1.0.0.jpskin") -Destination (Join-Path $stage "skins")
$archive = Join-Path $outputRoot "Juno-Pulsar-Desktop-$version-windows-x64-portable.zip"
Compress-Archive -LiteralPath $stage -DestinationPath $archive -Force
Get-FileHash -LiteralPath $archive -Algorithm SHA256 | ForEach-Object { "$($_.Hash.ToLower())  $([IO.Path]::GetFileName($_.Path))" } | Set-Content -LiteralPath (Join-Path $outputRoot "SHA256SUMS.txt")
Write-Output $archive
