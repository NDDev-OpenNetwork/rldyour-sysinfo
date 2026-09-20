# rldyour-sysinfo — end-to-end protocol check against a live daemon on Windows.
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# CPython on Windows builds without socket.AF_UNIX, so the wire protocol is
# exercised through .NET's UnixDomainSocketEndPoint — the same Winsock AF_UNIX
# transport the daemon serves. Two samples must arrive; the second must carry
# a CPU figure, because the first can legitimately be null while the counters
# establish a baseline.
$ErrorActionPreference = 'Stop'

$base = $env:LOCALAPPDATA
if (-not $base) { $base = $env:TEMP }
$path = Join-Path $base 'rldyour-sysinfo\rldyour-sysinfo.sock'

$socket = [System.Net.Sockets.Socket]::new(
    [System.Net.Sockets.AddressFamily]::Unix,
    [System.Net.Sockets.SocketType]::Stream,
    [System.Net.Sockets.ProtocolType]::Unspecified)
$socket.ReceiveTimeout = 30000
$socket.Connect([System.Net.Sockets.UnixDomainSocketEndPoint]::new($path))
[void]$socket.Send([System.Text.Encoding]::UTF8.GetBytes('{"interval":0}' + "`n"))

$reader = [System.IO.StreamReader]::new([System.Net.Sockets.NetworkStream]::new($socket))
try {
    for ($count = 1; $count -le 2; $count++) {
        $line = $reader.ReadLine()
        if ($null -eq $line) { throw 'the daemon closed the stream' }
        $sample = $line | ConvertFrom-Json
        if ($sample.v -ne 1) { throw "unexpected protocol version $($sample.v)" }
        Write-Output "sample $count`: cpu=$($sample.cpu.usage) mem=$($sample.memory.used) rx=$($sample.net.rx)"
        if ($count -eq 2 -and $null -eq $sample.cpu.usage) {
            throw 'cpu.usage was still null on the second sample'
        }
    }
    Write-Output 'e2e ok'
} finally {
    $reader.Close()
    $socket.Close()
}
