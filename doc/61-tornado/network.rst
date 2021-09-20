Network Common
``````````````

In order to keep Tornado independent from the network layer, the
*tornado_network_common* crate contains high level traits not bound to
any specific technology. These traits define the API for sending and
receiving Events and Actions across the network.

Simple Network
``````````````

The Simple Network is an implementation of the
*tornado_network_common::EventBus* that dispatches Events and Actions
on a single process without actually making network calls.
