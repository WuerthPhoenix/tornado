# Elasticsearch Executor

The Elasticsearch Executor is an executor that extracts data from a Tornado Action and sends
it to [Elasticsearch](https://www.elastic.co/guide/en/elasticsearch/reference/current/rest-apis.html).



## How It Works

This executor expects a Tornado Action to include the following elements in its payload:

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
