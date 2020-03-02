# Elasticsearch Executor

The Elasticsearch Executor is an executor that extracts data from a Tornado Action and sends
it to [Elasticsearch](https://www.elastic.co/guide/en/elasticsearch/reference/current/rest-apis.html).



## How It Works

This executor expects a Tornado Action that includes the following elements in its payload:

1. An __endpoint__ : The Elasticsearch endpoint which Tornado will call to create the Elasticsearch document.
1. An __index__ : The name of the Elasticsearch index in which the document will be created.
1. An __data__: The content of the document that will be sent to Elasticsearch.

An example of a valid Tornado Action is:
```json
{
    "id": "elasticsearch",
    "payload": {
        "endpoint": "http://localhost:9200",
        "index": "tornado-example",
        "data": {
            "user" : "kimchy",
            "post_date" : "2009-11-15T14:12:12",
            "message" : "trying out Elasticsearch"
        }
    }
}
```

The Elasticsearch Executor will create a new document in the specified Elasticsearch index for each action
executed. The specified index will be created if it does not already exist in Elasticsearch.


With this json the default authentication method created during the executor creation is used.
An action can define a specific authentication method as described in the next section.

## Elasticsearch authentication
When the Elasticsearch executor is created, a default authentication method can be specified.
In this case, if not differently specified by the action, this method will be used to authenticate to 
Elasticsearch. On the contrary, if a default method is not defined at creation time, then each action
that does not specify an authentication method will fail.

To use a specific authentication method the action should include the `auth` field with one of the following
authentication types:

* **None**: the client connects to Elasticsearch without authentication

    Example:
    ```json
    {
        "id": "elasticsearch",
        "payload": {
            "index": "tornado-example",
            "endpoint": "http://localhost:9200",
            "data": {
                "user": "myuser"
            },
            "auth": {
                "type": "None"
            }
        }
    }
    ```                 
  
* **PemCertificatePath**: the client connects to Elasticsearch using the PEM certificates read from the local
file system. When this method is used, the following information must be provided:
    * **certificate_path**: path to the public certificate accepted by Elasticsearch
    * **private_key_path**: path to the corresponding private key
    * **ca_certificate_path**: path to CA certificate needed to verify the identity of the Elasticsearch server 
    
    Example:
    ```json
    {
        "id": "elasticsearch",
        "payload": {
            "index": "tornado-example",
            "endpoint": "http://localhost:9200",
            "data": {
                "user": "myuser"
            },
            "auth": {
                "type": "PemCertificatePath",
                "certificate_path": "/path/to/tornado/conf/certs/tornado.crt.pem",
                "private_key_path": "/path/to/tornado/conf/certs/private/tornado.key.pem",
                "ca_certificate_path": "/path/to/tornado/conf/certs/root-ca.crt"
            }
        }
    }
    ```
