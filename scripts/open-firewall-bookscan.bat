@echo off
:: Ouvre le port TCP 7842 pour bookscan companion (Wi-Fi LAN)
netsh advfirewall firewall delete rule name="bookscan-companion" >nul 2>&1
netsh advfirewall firewall add rule name="bookscan-companion" dir=in action=allow protocol=TCP localport=7842 profile=any
netsh advfirewall firewall show rule name="bookscan-companion"
pause
