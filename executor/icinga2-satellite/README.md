# Icinga2 Satellite Executor

The Icinga2 Satellite Executor is an executor that extracts data from a Tornado Action and sends it 
to Icinga2 over their JSON-RPC API.


## How It Works

This executor expects a Tornado Action to include the following elements in its payload:

1. A __host__: The parameters of the Icinga2 satellite action.
1. A __service__ (optional): The parameters of the Icinga2 satellite action.
1. A __cr__ (optional): The result of the check, which is an object that is defined by icinga2 [here](https://icinga.com/docs/icinga-2/latest/doc/08-advanced-topics/#advanced-value-types-checkresult)
   1. none of these values are necessary. The state will default to `OK` and all other values will be left blank.
   1. It is not advised to set the `active` attribute, since all checks performed by tornado are passive for icinga2.

An example for a valid tornado action could look like this:
```json
{
   "id": "icinga2-satellite",
   "payload": {
      "host": "myhost",
      "service": "myservice",
      "cr": {
         "state": 2.0,
         "output": "CRITICAL: The service is no longer operational",
         "performance_data":["rta=0.067000ms;3000.000000;5000.000000;0.000000","pl=90%;80;100;0"]
      }
   }
}
```

The smallest possible tornado action is:
```json
{
   "id": "icinga2-satellite",
   "payload": {
      "host": "myhost"
   }
}
```
