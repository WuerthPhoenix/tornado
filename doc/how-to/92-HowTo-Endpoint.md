# <a id="tornado-howto-endpoint"></a> How To Use the Tornado Endpoint

This How To is intended to help you quickly configure the Tornado Endpoint server.
It is assumed that you are using a shell environment rather than the Tornado GUI.

As part of the standard Tornado installation within NetEye 4, a default Endpoint is
created.  You can check it's functioning properly via *curl*:
```
# curl 127.0.0.1:4748/monitoring
        <div>
            <h1>Available endpoints:</h1>
            <ul>
                <li><a href="/monitoring/ping">Ping</a></li>
            </ul>
        </div>
```

In general it's not safe from a security standpoint to have a server open to the world by default.
In fact, the Tornado Endpoint server is highly locked down, and if you were to try to connect to
it using the actual IP address, even from the server itself, you would find that it doesn't work
at all:
```
# curl http://203.0.113.51:4748/monitoring
curl: (7) Failed connect to 203.0.113.51:4748; Connection refused
```

The server process is started as part of the service *tornado.service*.  You can check the
parameters currently in use as follows:
```
# ps aux | grep tornado
root      6776  0.0  0.3 528980  7488 pts/0    Sl   10:02   0:00 /usr/bin/tornado --config-dir /neteye/shared/tornado/conf --logger-level=info --logger-stdout daemon
```

The IP address and port are not included, indicating the system is using the defaults, so we'll
need to configure the server to make it more useful.



## <a id="tornado-howto-endpoint-step1"></a> Step #1:  Setting Up the Endpoint Server

During installation, NetEye 4 automatically configures the Tornado Endpoint to start up with
the following defaults:
* **IP:**  127.0.0.1
* **Port:**  4748 (TCP)
* **Firewall:**  Enabled

You can change these defaults by editing the file that defines the service at
*/usr/lib/systemd/system/tornado.service*.  Here for instance, we've added options to change both
the IP and port:
```
[Unit]
Description=Tornado - Event Processing Engine

[Service]
Type=simple

#User=tornado
RuntimeDirectory=tornado
ExecStart=/usr/bin/tornado \
          --config-dir /neteye/shared/tornado/conf --logger-level=info --logger-stdout \
          daemon --web-server-ip=203.0.113.51 --web-server-port=4748
Restart=on-failure
RestartSec=3
# Other Restart options: or always, on-abort, etc

[Install]
WantedBy=neteye.target
```

Now we'll have to restart the Tornado service with our new parameters:
```
# systemctl daemon-reload
# systemctl restart tornado
```

Finally, if we want our endpoint to be visible from the outside, we'll need to open up the port
we declared in the firewall:
```
# firewall-cmd --zone=public --add-port=4748/tcp --permanent
# firewall-cmd --reload
```

<!-- Should we mention 0.0.0.0?  If so, what specifically about it? -->



## <a id="tornado-howto-endpoint-step2"></a>  Step #2:  Testing the Endpoint

You can now test your endpoint in a shell, both on the server itself as well as from other,
external clients:
```
# curl http://203.0.113.51:4748/monitoring
```

If you try with the browser, you should see the Endpoint page that currently consists of a link
to the "Ping" endpoint: 
```
http://203.0.113.51:4748/monitoring
```

If you click on it and see a response like the following, then you've successfully implemented
your Endpoint:
```
message	"pong - 2019-04-26T12:00:40.166378773+02:00"
```

Of course, you can do the same thing with curl, too:
```
# curl http://10.62.4.163:4741/monitoring/ping | jq .
{
  "message": "pong - 2019-04-26T14:06:04.573329037+02:00"
}
```
