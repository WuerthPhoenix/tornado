# <a id="tornado-howto-overview"></a> How to Use the Tornado How Tos

We assume that you are using a shell environment rather than the Tornado GUI.  If Tornado
is not already installed, you can install it as follows (the minimum Tornado version is 0.10.0):
```
# yum install tornado --enablerepo=neteye-extras
```

As a preliminary test, make sure that the Tornado service is up:
```
# neteye status
```

If you do not see any of the Tornado services in the list, then Tornado is not properly installed.
```
# systemctl daemon-reload
```

If instead the Tornado services are there, but marked DOWN, you will need to start them.
```
DOWN [3] tornado.service
DOWN [3] tornado_icinga2_collector.service
DOWN [3] tornado_webhook_collector.service
```

In either event, you should then start all NetEye services and check that they are running:
```
# neteye start
# neteye status
```

Alternatively, you can check the status of the Tornado service by itself:
```
# systemctl status tornado
... Active: active (running) ...
```

Finally, run a check on the default Tornado configuration directory.  You should see the
following output:
```
# tornado --config-dir=/neteye/shared/tornado/conf check
Check Tornado configuration
The configuration is correct.
```
