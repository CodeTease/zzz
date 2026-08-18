# Package Registry Setup Guide

`zzz` version `0.2.0`

This project automatically publishes packages to [Cloudsmith](https://cloudsmith.io/~codetease/tools/). 
To easily install `zzz` and receive future updates naturally through your system's package manager, run the relevant setup script for your environment.

## Linux Distributions

### Debian & Ubuntu (APT)
To configure the APT repository and install the package:
```bash
curl -1sLf 'https://dl.cloudsmith.io/public/codetease/tools/setup.deb.sh' | sudo -E distro=ubuntu codename=noble bash
sudo apt install zzz
```

### RHEL, CentOS & Fedora (RPM)
To configure the YUM/DNF repository and install the package:
```bash
curl -1sLf 'https://dl.cloudsmith.io/public/codetease/tools/setup.rpm.sh' | sudo -E distro=el codename=9 bash
sudo dnf install zzz
```

### Alpine Linux (APK)
To configure the APK repository and install the package:
```bash
curl -1sLf 'https://dl.cloudsmith.io/public/codetease/tools/setup.alpine.sh' | sudo -E distro=alpine codename=any-version bash
apk add zzz
```

### Arch Linux (PKGBUILD)
You can build and install the package using the provided `PKGBUILD` artifact from GitHub Releases.
```bash
curl -LO https://github.com/CodeTease/zzz/releases/download/v0.2.0/zzz-0.2.0-archlinux-pkgbuild.tar.gz
tar -xzf zzz-0.2.0-archlinux-pkgbuild.tar.gz
makepkg -si
```

## macOS & Linux (Homebrew)
You can install the package using our custom Homebrew tap:
```bash
brew tap CodeTease/homebrew-tap
brew install zzz
```

## Windows (NuGet)
To install the package via NuGet in PowerShell, register the Cloudsmith feed and install it:
```powershell
Register-PackageSource -Name 'codetease/tools' -ProviderName NuGet -Location "https://nuget.cloudsmith.io/codetease/tools/v3/index.json"
Install-Package zzz -Source 'codetease/tools'
```

Chocolatey:
```powershell
choco source add -n codetease/tools -s https://nuget.cloudsmith.io/codetease/tools/v3/index.json
choco install zzz -s codetease/tools
```

PowerShell:
```powershell
Register-PackageSource -Name 'codetease/tools' -ProviderName NuGet -Location "https://nuget.cloudsmith.io/codetease/tools/v2/" -Trusted
Register-PSRepository -Name 'codetease/tools' -SourceLocation "https://nuget.cloudsmith.io/codetease/tools/v2/" -InstallationPolicy 'trusted'

Install-Package zzz -Source 'codetease/tools'
# Or
Install-Module zzz -Repository 'codetease/tools'
```

## Windows (Scoop)
You can install the package using our custom Scoop bucket:
```powershell
scoop bucket add scoop-bucket https://github.com/CodeTease/scoop-bucket
scoop install scoop-bucket/zzz
```


