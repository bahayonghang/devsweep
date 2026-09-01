param(
    [Parameter(Mandatory = $true)]
    [string]$Root,
    [switch]$RestoreDeniedAcl,
    [switch]$ApplyDeniedAcl
)

$ErrorActionPreference = 'Stop'

if (-not ('DevSweepAnalyzeFixtureNative' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.IO;
using System.Runtime.InteropServices;

public static class DevSweepAnalyzeFixtureNative
{
    private const uint SddlRevision1 = 1;
    private const uint DaclSecurityInformation = 0x00000004;
    private const uint FsctlSetSparse = 590020;

    [StructLayout(LayoutKind.Sequential)]
    private struct SecurityAttributes
    {
        public uint Length;
        public IntPtr SecurityDescriptor;
        [MarshalAs(UnmanagedType.Bool)] public bool InheritHandle;
    }

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool ConvertStringSecurityDescriptorToSecurityDescriptor(
        string text, uint revision, out IntPtr descriptor, IntPtr descriptorSize);

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool SetFileSecurity(
        string path, uint securityInformation, IntPtr descriptor);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool CreateDirectory(string path, ref SecurityAttributes attributes);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr LocalFree(IntPtr memory);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool DeviceIoControl(
        IntPtr device, uint controlCode, IntPtr input, uint inputSize,
        IntPtr output, uint outputSize, out uint bytesReturned, IntPtr overlapped);

    private static IntPtr Descriptor(string sddl)
    {
        IntPtr descriptor;
        if (!ConvertStringSecurityDescriptorToSecurityDescriptor(
            sddl, SddlRevision1, out descriptor, IntPtr.Zero))
            throw new Win32Exception(Marshal.GetLastWin32Error());
        return descriptor;
    }

    public static void CreateDirectoryWithDacl(string path, string sddl)
    {
        IntPtr descriptor = Descriptor(sddl);
        try
        {
            var attributes = new SecurityAttributes {
                Length = (uint)Marshal.SizeOf<SecurityAttributes>(),
                SecurityDescriptor = descriptor,
                InheritHandle = false
            };
            if (!CreateDirectory(path, ref attributes))
                throw new Win32Exception(Marshal.GetLastWin32Error());
        }
        finally { LocalFree(descriptor); }
    }

    public static void SetDirectoryDacl(string path, string sddl)
    {
        IntPtr descriptor = Descriptor(sddl);
        try
        {
            if (!SetFileSecurity(path, DaclSecurityInformation, descriptor))
                throw new Win32Exception(Marshal.GetLastWin32Error());
        }
        finally { LocalFree(descriptor); }
    }

    public static void CreateSparseFile(string path, long logicalBytes)
    {
        using (var stream = new FileStream(path, FileMode.CreateNew, FileAccess.ReadWrite, FileShare.ReadWrite | FileShare.Delete))
        {
            uint returned;
            if (!DeviceIoControl(stream.SafeFileHandle.DangerousGetHandle(), FsctlSetSparse,
                IntPtr.Zero, 0, IntPtr.Zero, 0, out returned, IntPtr.Zero))
                throw new Win32Exception(Marshal.GetLastWin32Error());
            stream.SetLength(logicalBytes);
        }
    }
}
'@
}

$fullRoot = [System.IO.Path]::GetFullPath($Root)
$deniedPath = Join-Path $fullRoot 'denied'
$allowDacl = 'D:P(A;OICI;FA;;;WD)(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)'
$denyReadDacl = 'D:P(D;OICI;GRGX;;;WD)(A;OICI;FA;;;WD)(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)'

if ($RestoreDeniedAcl) {
    if (-not (Test-Path -LiteralPath $deniedPath)) {
        throw "Denied fixture path does not exist: $deniedPath"
    }
    [DevSweepAnalyzeFixtureNative]::SetDirectoryDacl($deniedPath, $allowDacl)
    [pscustomobject]@{ root = $fullRoot; denied_acl = 'restored' } | ConvertTo-Json -Compress
    return
}

if ($ApplyDeniedAcl) {
    if (-not (Test-Path -LiteralPath $deniedPath)) {
        throw "Denied fixture path does not exist: $deniedPath"
    }
    [DevSweepAnalyzeFixtureNative]::SetDirectoryDacl($deniedPath, $denyReadDacl)
    [pscustomobject]@{ root = $fullRoot; denied_acl = 'generic_read_execute_denied' } | ConvertTo-Json -Compress
    return
}

if (Test-Path -LiteralPath $fullRoot) {
    throw "Fixture root already exists: $fullRoot"
}
$parent = Split-Path -Parent $fullRoot
if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
    throw "Fixture parent does not exist: $parent"
}

[System.IO.Directory]::CreateDirectory($fullRoot) | Out-Null
[System.IO.Directory]::CreateDirectory((Join-Path $fullRoot 'alpha')) | Out-Null
[System.IO.Directory]::CreateDirectory((Join-Path $fullRoot 'beta')) | Out-Null
[System.IO.File]::WriteAllBytes((Join-Path $fullRoot 'alpha\one.bin'), [byte[]](1, 2, 3))
[System.IO.File]::WriteAllBytes((Join-Path $fullRoot 'beta\two.bin'), [byte[]](4, 5, 6, 7, 8))

[DevSweepAnalyzeFixtureNative]::CreateSparseFile((Join-Path $fullRoot 'sparse-logical.bin'), 8388608)
[DevSweepAnalyzeFixtureNative]::CreateDirectoryWithDacl($deniedPath, $allowDacl)
[System.IO.File]::WriteAllBytes((Join-Path $deniedPath 'excluded.bin'), [byte[]]::new(65536))
[DevSweepAnalyzeFixtureNative]::SetDirectoryDacl($deniedPath, $denyReadDacl)

[pscustomobject]@{
    root = $fullRoot
    expected_nodes = 7
    expected_root_lower_bound_bytes = 8388616
    sparse_logical_bytes = 8388608
    denied_content_bytes_excluded = 65536
    denied_acl = 'generic_read_execute_denied'
} | ConvertTo-Json -Compress
