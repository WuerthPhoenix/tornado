# <a id="tornado-howto-api"></a> How To Use the Tornado Self-Monitoring API

This How To is intended to help you quickly configure the Tornado self-monitoring API server.

Before continuing, you should first check the
[prerequisites for Tornado](/neteye/doc/module/tornado/chapter/tornado-howto-overview).

The self-monitoring API server is created as part of the standard Tornado installation within
NetEye 4.  You can check it is functioning properly via *curl*:
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
In fact, the Tornado self-monitoring API server is highly locked down, and if you were to try to
connect to it using the server's external network address, or even from the non-loopback address
on the server itself, you would find that it doesn't work at all:
```
# curl http://192.0.2.51:4748/monitoring
curl: (7) Failed connect to 192.0.2.51:4748; Connection refused
```

The server process is started as part of the service *tornado.service*.  You can check the
parameters currently in use as follows:
```
# ps aux | grep tornado
root      6776  0.0  0.3 528980  7488 pts/0    Sl   10:02   0:00 /usr/bin/tornado --config-dir /neteye/shared/tornado/conf --logger-level=info --logger-stdout daemon
```

The IP address and port are not included, indicating the system is using the defaults, so we'll
need to configure the server to make it more useful.



## <a id="tornado-howto-api-step1"></a> Step #1:  Setting Up the Self-Monitoring API Server

During installation, NetEye 4 automatically configures the Tornado self-monitoring API server
to start up with the following defaults:
* **IP:**  127.0.0.1
* **Port:**  4748 (TCP)
* **Firewall:**  Enabled

The file that defines the service can be found at */usr/lib/systemd/system/tornado.service*:
```
[Unit]
Description=Tornado - Event Processing Engine

[Service]
Type=simple

#User=tornado
RuntimeDirectory=tornado
ExecStart=/usr/bin/tornado \
          --config-dir /neteye/shared/tornado/conf --logger-level=info --logger-stdout \
          daemon
Restart=on-failure
RestartSec=3
# Other Restart options: or always, on-abort, etc

[Install]
WantedBy=neteye.target
```

If you want to change the default address and port, you shouldn't just modify that file directly,
since any changes would disappear after the next package update.  Instead, you can modify the
override file at */etc/systemd/system/tornado.service.d/neteye.conf*, or create a reverse proxy
in Apache, creating a */tornado/* * route that forwards requests to the *localhost* on the
desired port.
```
ExecStart=/usr/bin/tornado \
          --config-dir /neteye/shared/tornado/conf --logger-level=info --logger-stdout \
          daemon --web-server-ip=192.0.2.51 --web-server-port=4748
```

Now we'll have to restart the Tornado service with our new parameters:
```
# systemctl daemon-reload
# systemctl restart tornado
```

Finally, if we want our REST API to be visible externally, we'll need to either open up the port
we just declared in the firewall, or use the reverse proxy described above.  Otherwise, connection
requests to the API server will be refused.



## <a id="tornado-howto-api-step2"></a>  Step #2:  Testing the Self-Monitoring API

You can now test your REST API in a shell, both on the server itself as well as from other,
external clients:
```
# curl http://192.0.2.51:4748/monitoring
```

If you try with the browser, you should see the self-monitoring API page that currently consists
of a link to the "Ping" endpoint: 
```
http://192.0.2.51:4748/monitoring
```

If you click on it and see a response like the following, then you have successfully configured
your self-monitoring API server:
```
message	"pong - 2019-04-26T12:00:40.166378773+02:00"
```

Of course, you can do the same thing with curl, too:
```
# curl http://192.0.2.51:4748/monitoring/ping | jq .
{
  "message": "pong - 2019-04-26T14:06:04.573329037+02:00"
}
```
